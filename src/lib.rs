#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::process::Stdio;
use std::usize;

use libnss::group::Group;
use libnss::group::GroupHooks;
use libnss::interop::Response;
use libnss::libnss_group_hooks;
use libnss::libnss_passwd_hooks;
use libnss::libnss_shadow_hooks;
use libnss::passwd::Passwd;
use libnss::passwd::PasswdHooks;
use libnss::shadow::Shadow;
use libnss::shadow::ShadowHooks;

use shlex;

//////////////
//  Macros  //
//////////////

/** Print out a message if debug is true */
macro_rules! debug_print {
    ( $message:expr, $debug: ident ) => {
        if $debug {
            eprintln!("{}", $message);
        }
    };
}

/** Validate config.json
- If /etc/libnss_shim/config.json exists and is valid JSON, save the deserialized data and debug setting
- If config.json does not exist, is unreadable, or is not valid JSON, return with NSS code Unavail */
macro_rules! validate_config {
    ($deser: ident, $debug: ident) => {
        let $deser : serde_json::Value;
        let $debug : bool;
        match File::open("/etc/libnss_shim/config.json") {
            Ok(mut file) => {
                let mut content = String::new();
                // Read the file into a String (the file is closed automatically at the end of this scope)
                match file.read_to_string(&mut content) {
                    Ok(_) => {
                        // Check if it is actually valid formatted JSON using the fast IgnoredAny strategy
                        match serde_json::from_str::<serde::de::IgnoredAny>(&content) {
                            Ok(_) => {
                                // This appears to be valid JSON, so deserialize using the slower method
                                let deser: serde_json::Value = match serde_json::from_str(&content){
                                    Ok(deser) => deser,
                                    Err(_) => {
                                        // The JSON is actually invalid when parsed using the slower method
                                        // Unsure if this is actually possible to reach if checked with IgnoredAny, but doesn't hurt
                                        return Response::Unavail;
                                    }
                                };
                                // Check first if it is a valid object with expected structure (object at root with str key)
                                match deser.as_object() {
                                    Some(result) => {
                                        // Ensure it is not empty
                                        if result.len() < 1 {
                                            return Response::Unavail;
                                        }
                                        // Check debug setting
                                        if result.keys().any(| x| x == "debug") {
                                            match result["debug"].as_bool() {
                                                Some(debug_bool) => {
                                                    $debug = debug_bool;
                                                },
                                                _ => {
                                                    // A boolean is expected
                                                    return Response::NotFound;
                                                }
                                            }
                                        } else {
                                            $debug = false;
                                        }
                                        // Validate that it has the databases key, which is required
                                        if !result.keys().any(| x| x == "databases") {
                                            debug_print!(format!("No databases in config"), $debug);
                                            return Response::Unavail;
                                        }
                                        // It is valid, so return the deserialized JSON
                                        $deser = deser;
                                    }
                                    _ => {
                                        //JSON is not formatted as expected (no object at root)
                                        return Response::TryAgain;
                                    }
                                };
                            }
                            Err(_) => {
                                // This is not valid JSON
                                return Response::Unavail;
                            }
                        };
                    },
                    Err(_) => {
                        // The file cannot be read
                        return Response::Unavail;
                    }
                }
            },
            Err(_) => {
                // The file cannot be opened
                return Response::Unavail;
            },
        }
    };
}

/** Validate command output
- If JSON, set an Option with the deserialized response data
- If not JSON, set an Option with None
- If invalid or blank, return the appropriate NSS code */
macro_rules! validate_response {
    ( $json: ident, $option: ident, $debug: ident ) => {
        let $option : Option<serde_json::Value>;
        // Check if it is actually valid formatted JSON using the fast IgnoredAny strategy
        match serde_json::from_str::<serde::de::IgnoredAny>(&$json) {
            Ok(_) => {
                // This appears to be valid JSON, so deserialize using the slower method
                let deser: serde_json::Value = match serde_json::from_str(&$json){
                    Ok(deser) => deser,
                    Err(e) => {
                        // Unsure if this is actually possible to reach if checked with IgnoredAny, but doesn't hurt
                        debug_print!(format!("JSON response: {} is invalid: {}", &$json, e), $debug);
                        return Response::TryAgain;
                    }
                };
                // Check first if it is a valid object with expected structure (object at root with str key)
                match deser.as_object() {
                    Some(result) => {
                        // If it is valid but has an empty response, that means nothing was found
                        if result.len() < 1 {
                            debug_print!(format!("Nothing found in JSON: {}", &$json), $debug);
                            return Response::NotFound;
                        }
                        // Validate that it has at least one addressable key
                        match result.keys().nth(0) {
                            Some(x) => {x},
                            _ => {
                                debug_print!(format!("No addressable keys found in JSON: {}", &$json), $debug);
                                return Response::TryAgain
                            }
                        };
                        // It is valid, so set the deserialized JSON as the Option contents
                        $option = Some(deser);
                    }
                    _ => {
                        // The JSON is not formatted as expected
                        debug_print!(format!("JSON not formatted as expected: {}", &$json), $debug);
                        return Response::TryAgain;
                    }
                };
            }
            Err(e) => {
                // This is not valid JSON, so assume it is in passwd type string format
                // If it has less than 4 chars after trimming whitespace, then it is considered NotFound
                if $json.trim().len() < 4 {
                    debug_print!(format!("Response is not json: {}, decoded: {}", &$json, e), $debug);
                    return Response::NotFound;
                }
                // It appears to be in passwd file style format, so set the option to None
                $option = None;
            }
        };
    }
}

/** Parse the config JSON for a specific function
- If the expected function/database is not present, return NSS code NotFound
- If the function is there but no command is defined, return NSS code Unavail
- If the command is not valid/parseable, return NSS code Unavail
- If all checks pass, return a Vec of command/args, a Map of env vars, and a working directory */
macro_rules! parse_config {
    ($config_deser: ident, $target_db: ident, $target_function: ident, $command: ident, $env_vars: ident, $dir: ident, $debug: ident ) => {
        // Define the 'path' in the json to get to the command from the root
        let command_path: [String; 5] = [
            "databases".to_string(),
            $target_db,
            "functions".to_string(),
            $target_function,
           "command".to_string()
        ];
        // Define the indexes of the levels in command_path to check for env/workdir
        // 0 is the same level as the first entry, i.e. the root
        // It is in reverse order to prioritize function > group > global
        let levels = [4, 2, 0];
        let mut $command : Vec<String> = Vec::new();
        let mut $env_vars : HashMap<String, String> = HashMap::new();
        let mut $dir = "".to_string();

        // Go through the json and validate it against the expected path
        let mut deser_obj = &$config_deser;
        for (i, path_entry) in command_path.iter().enumerate(){
            // First, check if the JSON matches the expected command_path step
            match &deser_obj.as_object() {
                Some(x) => {
                    if !x.keys().any(|x| x == path_entry) {
                        // Expected config key does not exist in the expected path
                        if path_entry == "command" || path_entry == "functions" {
                            // the "command" and "functions" keys should exist if we got that far in the path
                            // databases is also required but it is already checked during config validation
                            debug_print!(format!("config.json does not contain required key: {} for command path: {:?}", path_entry, command_path), $debug);
                            return Response::Unavail;
                        }
                        // Otherwise it's fine since the command/group was not defined
                        return Response::NotFound;
                    }
                },
                _ => {
                    // An object/map is expected
                    debug_print!(format!("config.json not formatted as expected"), $debug);
                    return Response::Unavail;
                }
            }
            // The JSON matches the expected command path, so set the next level
            deser_obj = &deser_obj[path_entry];
            // If we're at the end of the path, that means we should be at the command
            if i+1 == command_path.len() {
                // Check for custom 'env' and 'workdir'
                // Check each of the expected levels to see if env/workdir are set
                for level in levels {
                    // Go through the json to reach the target level, if it is not at 0
                    let mut level_json = &$config_deser;
                    if level > 0 {
                        for (i, command_level) in command_path.iter().enumerate(){
                            if i == level {
                                break;
                            }
                            level_json = &level_json[command_level];
                        }
                    }
                    // Only set a custom env/dir variable if it has not been set yet
                    if $env_vars.len() == 0 &&  level_json.as_object().unwrap().keys().any(|x| x == "env"){
                        match level_json["env"].as_object() {
                            Some(env_obj) => {
                                for env_var in env_obj.keys() {
                                    match level_json["env"][env_var].as_str() {
                                        Some(env_string) => {
                                            $env_vars.insert(env_var.to_string(), env_string.to_string());
                                        },
                                        _ => {
                                            // A string is expected
                                            debug_print!(format!("env variables in config.json must be strings"), $debug);
                                            return Response::Unavail
                                        }
                                    }
                                }
                            },
                            _ => {
                                debug_print!(format!("env variables in config.json must be in an object/map"), $debug);
                                // An object/map is expected
                                return Response::Unavail
                            }
                        }
                    }
                    if $dir == "".to_string() && level_json.as_object().unwrap().keys().any(|x| x == "workdir"){
                        match level_json["workdir"].as_str() {
                            Some(workdir_string) => {
                                $dir = workdir_string.to_string();
                            },
                            _ => {
                                // A string is expected
                                debug_print!(format!("workdir in config.json must be a string"), $debug);
                                return Response::Unavail
                            }
                        }
                    }
                    // If they've both been set, stop looking
                    if $env_vars.len() > 0 && $dir != "".to_string() {
                        break;
                    }
                }
                // Try to split the command with shlex - if it fails, then this command is invalid or unreadable
                $command = match shlex::split(&deser_obj.as_str().unwrap().to_string()) {
                    Some(x) => {
                        if x.len() < 1 {
                            // If there is no actual command, then this is invalid
                            debug_print!(format!("No command defined in config.json for path {:?}", command_path), $debug);
                            return Response::Unavail;
                        }
                        x
                    },
                    _ => {
                        debug_print!(format!("Unable to split command/args: {} defined for path: {:?}", &deser_obj.as_str().unwrap().to_string(), command_path ), $debug);
                        return Response::Unavail;
                    }
                };
            };
        }
    }
}

/** Parse a line in /etc/group format into a Group object
- If invalid, return the appropriate NSS code
- If missing entries, they will be replaced with blanks */
macro_rules! parse_group_format {
    ( $entries: ident, $debug: ident ) => {
        Group {
            name: match $entries.next() {
                Some(s) => s.to_string(),
                _ => {
                    debug_print!(format!("Unable to parse name for group"), $debug);
                    return Response::TryAgain;
                }
            },
            passwd: match $entries.next() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            },
            gid: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => {
                    debug_print!(format!("Unable to parse gid for group"), $debug);
                    return Response::TryAgain;
                }
            },
            members: match $entries.next() {
                Some(s) => {
                    let mut members: Vec<String> = Vec::new();
                    for user in s.split(",") {
                        members.push(user.to_string());
                    }
                    members
                }
                _ => {
                    let members: Vec<String> = Vec::new();
                    members
                }
            },
        }
    };
}

/** Parse a line in /etc/passwd format into a Passwd object
- If invalid, return the appropriate NSS code
- If missing entries, they will be replaced with blanks */
macro_rules! parse_passwd_format {
    ( $ entries: ident, $debug: ident ) => {
        Passwd {
            name: match $entries.next() {
                Some(s) => s.to_string(),
                _ => {
                    debug_print!(format!("Unable to parse name for passwd"), $debug);
                    return Response::TryAgain;
                }
            },
            passwd: match $entries.next() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            },
            uid: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => {
                    debug_print!(format!("Unable to parse uid for passwd"), $debug);
                    return Response::TryAgain;
                }
            },
            gid: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => {
                    debug_print!(format!("Unable to parse gid for passwd"), $debug);
                    return Response::TryAgain;
                }
            },
            gecos: match $entries.next() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            },
            dir: match $entries.next() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            },
            shell: match $entries.next() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            },
        }
    };
}

/** Parse a line in /etc/shadow format into a Shadow object
- If invalid, return the appropriate NSS code
- If missing entries, they will be replaced with blanks */
macro_rules! parse_shadow_format {
    ( $entries: ident, $debug: ident ) => {
        Shadow {
            name: match $entries.next() {
                Some(s) => s.to_string(),
                _ => {
                    debug_print!(format!("Uname to parse name for shadow"), $debug);
                    return Response::TryAgain;
                }
            },
            passwd: match $entries.next() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            },
            last_change: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => -1,
            },
            change_min_days: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => -1,
            },
            change_max_days: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => -1,
            },
            change_warn_days: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => -1,
            },
            change_inactive_days: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => -1,
            },
            expire_date: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => -1,
            },
            reserved: match $entries.next().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                _ => usize::MAX,
            },
        }
    };
}

/** Parse a JSON group object
- If invalid, return the appropriate NSS code
- If missing entries, they will be replaced with blanks */
macro_rules! parse_group_json {
    ( $group_entry: ident, $deser: ident, $group: ident, $debug: ident ) => {
        let name = $group_entry.to_string();
        let passwd;
        if $deser[$group_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "passwd")
        {
            passwd = $deser[$group_entry]["passwd"]
                .as_str()
                .unwrap_or_else(|| "")
                .to_string();
        } else {
            passwd = "".to_string();
        }
        let gid = match $deser[$group_entry]["gid"].as_u64() {
            Some(x) => x,
            _ => {
                debug_print!(format!("Unable to parse gid for group JSON"), $debug);
                return Response::TryAgain;
            }
        } as u32;
        let mut members: Vec<String> = Vec::new();
        if $deser[$group_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "members")
        {
            for member in match $deser[$group_entry]["members"].as_array() {
                Some(x) => x,
                _ => {
                    debug_print!(
                        format!("Unable to parse group members for group JSON"),
                        $debug
                    );
                    return Response::TryAgain;
                }
            } {
                members.push(
                    match member.as_str() {
                        Some(x) => x,
                        _ => {
                            debug_print!(
                                format!("Unable to parse group member for group JSON"),
                                $debug
                            );
                            return Response::TryAgain;
                        }
                    }
                    .to_string(),
                );
            }
        }
        let $group = Group {name, passwd, gid, members};
    };
}

/** Parse a JSON passwd object
- If invalid, return the appropriate NSS code
- If missing entries, they will be replaced with blanks */
macro_rules! parse_passwd_json {
    ( $passwd_entry: ident, $deser: ident, $passwd: ident, $debug: ident ) => {
        let name = $passwd_entry.to_string();
        let mut passwd = "".to_string();
        if $deser[$passwd_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "passwd")
        {
            passwd = match $deser[$passwd_entry]["passwd"].as_str() {
                Some(x) => x,
                _ => {
                    debug_print!(
                        format!("Unable to parse passwd: {} for passwd JSON", passwd),
                        $debug
                    );
                    return Response::TryAgain;
                }
            }
            .to_string();
        }
        let uid = match $deser[$passwd_entry]["uid"].as_u64() {
            Some(x) => x,
            _ => {
                debug_print!(format!("Unable to parse uid for passwd JSON"), $debug);
                return Response::TryAgain;
            }
        } as u32;
        let gid = match $deser[$passwd_entry]["gid"].as_u64() {
            Some(x) => x,
            _ => {
                debug_print!(format!("Unable to parse gid for passwd JSON"), $debug);
                return Response::TryAgain;
            }
        } as u32;
        let mut gecos = "".to_string();
        if $deser[$passwd_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "gecos")
        {
            gecos = match $deser[$passwd_entry]["gecos"].as_str() {
                Some(x) => x,
                _ => {
                    debug_print!(
                        format!("Unable to parse gecos: {} for passwd JSON", gecos),
                        $debug
                    );
                    return Response::TryAgain;
                }
            }
            .to_string();
        }
        let mut dir = "".to_string();
        if $deser[$passwd_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "dir")
        {
            dir = match $deser[$passwd_entry]["dir"].as_str() {
                Some(x) => x,
                _ => {
                    debug_print!(
                        format!("Unable to parse dir: {} for passwd JSON", dir),
                        $debug
                    );
                    return Response::TryAgain;
                }
            }
            .to_string();
        }
        let mut shell = "".to_string();
        if $deser[$passwd_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "shell")
        {
            shell = match $deser[$passwd_entry]["shell"].as_str() {
                Some(x) => x,
                _ => {
                    debug_print!(
                        format!("Unable to parse shell: {} for passwd JSON", shell),
                        $debug
                    );
                    return Response::TryAgain;
                }
            }
            .to_string();
        }
        let $passwd = Passwd {name, passwd, uid, gid, gecos, dir, shell};
    };
}

/** Parse a JSON shadow object
- If invalid, return the appropriate NSS code
- If missing entries, they will be replaced with blanks */
macro_rules! parse_shadow_json {
    ( $shadow_entry: ident, $deser: ident, $shadow: ident , $debug: ident) => {
        let name = $shadow_entry.to_string();
        let mut passwd = "".to_string();
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "passwd")
        {
            passwd = match $deser[$shadow_entry]["passwd"].as_str() {
                Some(x) => x,
                _ => {
                    debug_print!(
                        format!("Unable to parse passwd: {} for shadow JSON", passwd),
                        $debug
                    );
                    return Response::TryAgain;
                }
            }
            .to_string();
        }
        let mut last_change = -1;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "last_change")
        {
            last_change = match $deser[$shadow_entry]["last_change"].as_i64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "last_change cannot be parsed as isize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!(
                            "Unable to parse last_change: {} for shadow JSON",
                            last_change
                        ),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let mut change_min_days = -1;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "change_min_days")
        {
            change_min_days = match $deser[$shadow_entry]["change_min_days"].as_i64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "change_min_days cannot be parsed as isize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!(
                            "Unable to parse change_min_days: {} for shadow JSON",
                            change_min_days
                        ),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let mut change_max_days = -1;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "change_max_days")
        {
            change_max_days = match $deser[$shadow_entry]["change_max_days"].as_i64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "change_max_days cannot be parsed as isize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!(
                            "Unable to parse change_max_days: {} for shadow JSON",
                            change_max_days
                        ),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let mut change_warn_days = -1;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "change_warn_days")
        {
            change_warn_days = match $deser[$shadow_entry]["change_warn_days"].as_i64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "change_warn_days cannot be parsed as isize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!(
                            "Unable to parse change_warn_days: {} for shadow JSON",
                            change_warn_days
                        ),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let mut change_inactive_days = -1;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "change_inactive_days")
        {
            change_inactive_days = match $deser[$shadow_entry]["change_inactive_days"].as_i64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "change_inactive_days cannot be parsed as isize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!(
                            "Unable to parse change_inactive_days: {} for shadow JSON",
                            change_inactive_days
                        ),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let mut expire_date = -1;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "expire_date")
        {
            expire_date = match $deser[$shadow_entry]["expire_date"].as_i64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "expire_date cannot be parsed as isize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!(
                            "Unable to parse expire_date: {} for shadow JSON",
                            expire_date
                        ),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let mut reserved = usize::MAX;
        if $deser[$shadow_entry]
            .as_object()
            .unwrap()
            .keys()
            .any(|x| x == "reserved")
        {
            reserved = match $deser[$shadow_entry]["reserved"].as_u64() {
                Some(x) => match x.try_into(){
                    Ok(y) => { y },
                    Err(e) => {
                        debug_print!(
                            format!(
                                "expire_date cannot be parsed as usize: {}",
                                e
                            ),
                            $debug
                        );
                        return Response::TryAgain;
                    }
                },
                _ => {
                    debug_print!(
                        format!("Unable to parse reserved: {} for shadow JSON", reserved),
                        $debug
                    );
                    return Response::TryAgain;
                }
            };
        }
        let $shadow = Shadow {
            name,
            passwd,
            last_change,
            change_min_days,
            change_max_days,
            change_warn_days,
            change_inactive_days,
            expire_date,
            reserved,
        };
    };
}

/** Run the command&args with the given workdir and env vars.
- If the code tuple has non-empty strings, instances of the first will be replaced by the second in args and env vars
- If the command has a runtime error, return NSS code TryAgain
- If the command succeeds, return the trimmed output */
macro_rules! run_command_capture_output {
    ($command: ident, $env_vars: ident, $dir: ident, $code: ident, $output: ident, $debug: ident) => {
        // Create the runnable command using the first item in the args (i.e. the base command to run)
        let mut runnable_command = Command::new(&$command[0]);
        // Add arguments to the command by iterating through and replacing any of the special codes
        for (i, arg) in $command.iter().enumerate() {
            if i > 0 {
                runnable_command.arg(str::replace(arg, &$code.0, &$code.1));
            }
        }
        // Add environment variables to the command by iterating through and replacing any of the special codes
        for env_var in $env_vars.keys() {
            runnable_command.env(str::replace(env_var, &$code.0, &$code.1), str::replace(&$env_vars[env_var], &$code.0, &$code.1));
        }
        // Set working directory for running command, if present
        if $dir.len() > 0 {
            runnable_command.current_dir($dir);
        }
        // Run the command and capture the output, returning NSS code TryAgain if something goes wrong
        let $output = match String::from_utf8(
            match runnable_command
                .stdout(Stdio::piped())
                .output() {
                Ok(x) => x.stdout,
                _ => {
                    debug_print!(format!("Runtime error for command"), $debug);
                    return Response::TryAgain
                }
            }
        ) {
            Ok(mut x) => {
                x = x.trim().to_string();
                x
            }
            _ => {
                debug_print!(format!("Unable to capture output from command as string"), $debug);
                return Response::TryAgain
            }
        };
    };
}

///////////////////
//  Group hooks  //
///////////////////

struct ShimGroup;
libnss_group_hooks!(shim, ShimGroup);

impl GroupHooks for ShimGroup {
    fn get_all_entries() -> Response<Vec<Group>> {
        // Ensure the configuration is usable
        validate_config!(config_deser, debug);
        // Get the command data for this particular db and function
        let group = "group".to_string();
        let function = "get_all_entries".to_string();
        parse_config!(config_deser, group, function, command, env_vars, dir, debug);
        // Since this function does not have a uid/gid/name parameter, set the code as blank
        let code = ("".to_string(), "".to_string());
        // Run the command defined in the config and capture the output as String
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        // Validate the command output and determine if if it is JSON or not
        validate_response!(output, option, debug);
        let mut group_vec: Vec<Group> = Vec::new();
        match option {
            Some(deser) => {
                // Parse as JSON
                for group_entry in deser.as_object().unwrap().keys() {
                    parse_group_json!(group_entry, deser, group, debug);
                    group_vec.push(group);
                }
            }
            _ => {
                // Parse as unix-style file format
                for line in output.trim().lines() {
                    if line.matches(":").count() < 3 {
                        debug_print!(
                            format!(
                                "Returned group data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(4, ':');
                    group_vec.push(parse_group_format!(entries, debug));
                }
            }
        };
        // Shouldn't ever be 0, but good to check
        if group_vec.len() > 0 {
            return Response::Success(group_vec);
        }
        debug_print!(format!("Returned group data is invalid"), debug);
        return Response::TryAgain;
    }

    fn get_entry_by_gid(gid: libc::gid_t) -> Response<Group> {
        validate_config!(config_deser, debug);
        let database = "group".to_string();
        let function = "get_entry_by_gid".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        // Set the code for gid
        let code = ("<$gid>".to_string(), gid.to_string());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        match option {
            Some(deser) => {
                // Return after first loop iteration since this is supposed to be a single entry
                for group_entry in deser.as_object().unwrap().keys() {
                    if gid != match deser[group_entry]["gid"].as_u64() {
                        Some(x) => x,
                        _ => {
                            debug_print!(
                                format!(
                                    "Returned group data: {} does not contain a valid gid",
                                    deser
                                ),
                                debug
                            );
                            return Response::TryAgain;
                        }
                    } as u32
                    {
                        debug_print!(
                            format!(
                                "Returned group data: {} does not contain a matching gid: {}",
                                deser,
                                gid
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    parse_group_json!(group_entry, deser, group, debug);
                    return Response::Success(group);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    if line.matches(":").count() < 3 {
                        debug_print!(
                            format!(
                                "Returned group data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(4, ':');
                    return Response::Success(parse_group_format!(entries, debug));
                }
                // Shouldn't happen since this implies the text had no lines
                debug_print!(
                    format!("Returned group data does not contain valid strings"),
                    debug
                );
                return Response::TryAgain;
            }
        };
        debug_print!(format!("gid: {} not found in group", gid), debug);
        return Response::NotFound;
    }

    fn get_entry_by_name(name: String) -> Response<Group> {
        validate_config!(config_deser, debug);
        let database = "group".to_string();
        let function = "get_entry_by_name".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        let code = ("<$name>".to_string(), name.clone());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        match option {
            Some(deser) => {
                for group_entry in deser.as_object().unwrap().keys() {
                    if code.1 != group_entry.to_string() {
                        debug_print!(
                            format!(
                                "Requested name: {} does not match returned name: {}",
                                code.1,
                                group_entry.to_string()
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    parse_group_json!(group_entry, deser, group, debug);
                    return Response::Success(group);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    if line.matches(":").count() < 3 {
                        debug_print!(
                            format!(
                                "Returned group data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(4, ':');
                    return Response::Success(parse_group_format!(entries, debug));
                }
                debug_print!(
                    format!("Returned group data does not match expected unix form"),
                    debug
                );
                return Response::TryAgain;
            }
        };
        debug_print!(format!("Name: {} not found in group", name), debug);
        return Response::NotFound;
    }
}

//////////////////
// Passwd hooks //
//////////////////

struct ShimPasswd;
libnss_passwd_hooks!(shim, ShimPasswd);

impl PasswdHooks for ShimPasswd {
    fn get_all_entries() -> Response<Vec<Passwd>> {
        validate_config!(config_deser, debug);
        let database = "passwd".to_string();
        let function = "get_all_entries".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        let code = ("".to_string(), "".to_string());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        let mut passwd_vec: Vec<Passwd> = Vec::new();
        match option {
            Some(deser) => {
                for passwd_entry in deser.as_object().unwrap().keys() {
                    parse_passwd_json!(passwd_entry, deser, passwd, debug);
                    passwd_vec.push(passwd);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    if line.matches(":").count() < 6 {
                        debug_print!(
                            format!(
                                "Returned passwd data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(7, ':');
                    passwd_vec.push(parse_passwd_format!(entries, debug));
                }
            }
        };
        if passwd_vec.len() > 0 {
            return Response::Success(passwd_vec);
        }
        debug_print!(format!("Returned passwd data is invalid"), debug);
        return Response::TryAgain;
    }

    fn get_entry_by_uid(uid: libc::uid_t) -> Response<Passwd> {
        validate_config!(config_deser, debug);
        let database = "passwd".to_string();
        let function = "get_entry_by_uid".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        let code = ("<$uid>".to_string(), uid.to_string());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        match option {
            Some(deser) => {
                for passwd_entry in deser.as_object().unwrap().keys() {
                    if uid != match deser[passwd_entry]["uid"].as_u64() {
                        Some(x) => x,
                        _ => {
                            debug_print!(
                                    format!(
                                        "Returned passwd data: {} does not contain a valid uid",
                                        deser
                                    ),
                                    debug
                                );
                            return Response::TryAgain;
                        }
                    } as u32
                    {
                        debug_print!(
                            format!(
                                "Returned passwd data: {} does not contain a matching uid: {}",
                                deser,
                                uid
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    parse_passwd_json!(passwd_entry, deser, passwd, debug);
                    return Response::Success(passwd);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    if line.matches(":").count() < 6 {
                        debug_print!(
                            format!(
                                "Returned passwd data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(7, ':');
                    return Response::Success(parse_passwd_format!(entries, debug));
                }
            }
        };
        debug_print!(format!("uid: {} not found in passwd", uid), debug);
        return Response::NotFound;
    }

    fn get_entry_by_name(name: String) -> Response<Passwd> {
        validate_config!(config_deser, debug);
        let database = "passwd".to_string();
        let function = "get_entry_by_name".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        let code = ("<$name>".to_string(), name.clone());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        match option {
            Some(deser) => {
                for passwd_entry in deser.as_object().unwrap().keys() {
                    if code.1 != passwd_entry.to_string() {
                        debug_print!(
                            format!(
                                "Requested name: {} does not match returned name: {}",
                                code.1,
                                passwd_entry.to_string()
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    parse_passwd_json!(passwd_entry, deser, passwd, debug);
                    return Response::Success(passwd);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    if line.matches(":").count() < 6 {
                        debug_print!(
                            format!(
                                "Returned passwd data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(7, ':');
                    return Response::Success(parse_passwd_format!(entries, debug));
                }
            }
        };
        debug_print!(format!("Name: {} not found in passwd", name), debug);
        return Response::NotFound;
    }
}

//////////////////
// Shadow hooks //
//////////////////

struct ShimShadow;
libnss_shadow_hooks!(shim, ShimShadow);

impl ShadowHooks for ShimShadow {
    fn get_all_entries() -> Response<Vec<Shadow>> {
        validate_config!(config_deser, debug);
        let database = "shadow".to_string();
        let function = "get_all_entries".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        let code = ("".to_string(), "".to_string());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        let mut shadow_vec: Vec<Shadow> = Vec::new();
        match option {
            Some(deser) => {
                for shadow_entry in deser.as_object().unwrap().keys() {
                    parse_shadow_json!(shadow_entry, deser, shadow, debug);
                    shadow_vec.push(shadow);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    // Allowing 7 instead of 8 since the last field ('reserved') is truly optional
                    if line.matches(":").count() < 7 {
                        debug_print!(
                            format!(
                                "Returned shadow data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(9, ':');
                    shadow_vec.push(parse_shadow_format!(entries, debug));
                }
            }
        };
        if shadow_vec.len() > 0 {
            return Response::Success(shadow_vec);
        }
        debug_print!(format!("Returned shadow data is invalid"), debug);
        return Response::TryAgain;
    }

    fn get_entry_by_name(name: String) -> Response<Shadow> {
        validate_config!(config_deser, debug);
        let database = "shadow".to_string();
        let function = "get_entry_by_name".to_string();
        parse_config!(config_deser, database, function, command, env_vars, dir, debug);
        let code = ("<$name>".to_string(), name.clone());
        run_command_capture_output!(command, env_vars, dir, code, output, debug);
        validate_response!(output, option, debug);
        match option {
            Some(deser) => {
                for shadow_entry in deser.as_object().unwrap().keys() {
                    if code.1 != shadow_entry.to_string() {
                        debug_print!(
                            format!(
                                "Requested name: {} does not match returned name: {}",
                                code.1,
                                shadow_entry.to_string()
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    parse_shadow_json!(shadow_entry, deser, shadow, debug);
                    return Response::Success(shadow);
                }
            }
            _ => {
                for line in output.trim().lines() {
                    // Allowing 7 instead of 8 since the last field ('reserved') is truly optional
                    if line.matches(":").count() < 7 {
                        debug_print!(
                            format!(
                                "Returned shadow data: {} does not match expected unix form",
                                line
                            ),
                            debug
                        );
                        return Response::TryAgain;
                    }
                    let mut entries = line.trim().splitn(9, ':');
                    return Response::Success(parse_shadow_format!(entries, debug));
                }
            }
        };
        debug_print!(format!("Name: {} not found in shadow", name), debug);
        return Response::NotFound;
    }
}
