# libnss_shim

Respond to [Name Service Switch](https://www.gnu.org/software/libc/manual/html_node/Name-Service-Switch.html) lookups
with the output of custom commands. Both JSON and the standard colon-separated *nix format are supported.

## Overview

`libnss_shim` is an adapter to make integration with NSS easier. It is an NSS/nsswitch service that runs commands
defined per-function in `config.json`. Commands can output responses to queries either in the typical colon-delimited
Unix format, or in JSON. The output of each command execution is parsed from `stdout` and validated before being passed
back to NSS (see the Commands section for details).

## Demonstration

![libnss_shim GIF demo](https://raw.githubusercontent.com/xenago/libnss_shim/main/doc/libnss_shim_demo.gif)

## Background

Custom [PAM](https://www.man7.org/linux/man-pages/man8/pam.8.html) modules alone are not enough to create a custom Linux
authentication process - integration with NSS is also required to inject custom user data to `group`/`passwd`/`shadow`
lookups earlier in the login flow.

In other words: NSS determines if an account exists, and PAM determines how an account can be accessed.

A good example of this is [SSSD](https://sssd.io), which leverages both NSS and PAM to enable seamless LDAP
authentication. Integrating directly with NSS can be difficult, so `libnss_shim` was created to allow any command that
can print to `stdout` in a supported format to be used with NSS.

## Installation

### Compatibility notes

- Tested on Debian-based GNU/Linux distributions
- Builds for `amd64` architecture
- If `.deb` packages are not supported on the desired target platform, `libnss_shim` might be usable if the `assets` as
  described in `Cargo.toml` are installed prior to running the `debian/postinst`  script, but this has not been tested
- To request support for a different configuration, please create a GitHub Issue

### Installation steps

1. Prepare the commands/software that will be triggered by `libnss_shim` (see the Commands section for details).

2. Download the latest release produced by GitHub Actions:
    ```
    wget https://github.com/xenago/libnss_shim/releases/download/1.0.2/libnss_shim_1.0.2_amd64.deb
    ```

3. Install it directly with `dpkg` or through `apt`:
    ```
    sudo dpkg -i libnss_shim_1.0.2_amd64.deb
    ```
   or
    ```
    sudo apt install ./libnss_shim_1.0.2_amd64.deb
    ```

4. Configure the shim by importing a custom `config.json`:
    ```
    sudo cp custom_config.json /etc/libnss_shim/config.json
    ```
   Using the preinstalled `config.json`, `libnss_shim` should have no effect, as the default configuration has commands
   defined that output nothing (see the Configuration section for details). Updates to the config take effect
   immediately and can be performed at any time after `libnss_shim` has been installed and used, without restarting.

5. By default, `shim` (meaning `libnss_shim`) is defined in `/etc/nsswitch.conf` as the final source for all supported
   databases. In that file, you can change the access order for each database's sources, remove `shim` from specific
   locations if not required, etc. Unlike `config.json`, `nsswitch.conf` is read only once per-process, so any software
   actively using it will need to be started or restarted.

   Rebooting the system is often the safest/easiest way to do this:
    ```
    sudo reboot
    ```

6. Perform NSS queries to validate the installation, for example using the built-in `getent` tool.

   Some sample commands:
    ```
    getent group
    getent passwd
    getent shadow
    getent group <groupname>
    ```

## Uninstallation

1. To remove `libnss_shim`, use either `dpkg` or `apt`:
   ```
   dpkg -r libnss_shim
   ```
   or
   ```
   sudo apt remove libnss_shim
   ```

2. If removal/deletion is performed, restarting affected applications is required. A system reboot is an effective way
   to do this:
   ```
   sudo reboot
   ```

## Configuration

Functions for 3 NSS databases are supported:

- `group`
    - `get_all_entries()`
    - `get_entry_by_gid(uint32 gid)`
    - `get_entry_by_name(str name)`
- `passwd`
    - `get_all_entries()`
    - `get_entry_by_uid(uint32 uid)`
    - `get_entry_by_name(str name)`
- `shadow`
    - `get_all_entries()`
    - `get_entry_by_name(str name)`

Codes can be used to insert relevant query data at runtime into the environment variables or launch arguments of
commands run by `libnss_shim`:

- `<$gid>`
- `<$name>`
- `<$uid>`

Using only that information, here is an extremely basic test example of `config.json` - one database is defined, `group`
, with just a single function, `get_all_entries`:

```
{
  "databases": {
    "group": {
      "functions": {
        "get_all_entries": {
          "command": "echo 'testgroup::1008:fake-username,another-user'"
        }
      }
    }
  }
}
```

The command defined for `get_all_entries` prints out a single line to `stdout`, describing a fake group
called `testgroup` with `gid=1008` and two members. That output is then captured by `libss_shim` and returned
to `NSS` whenever a call is made requesting all the group entries (e.g. `getent group`).

To support command execution, the following options can be set globally and overridden for specific databases and/or
functions:

- `"env"`: Add environment variables to the set inherited from `libnss_shim`
- `"workdir"`: Set the working directory before running the command

To enable debug printing to the user terminal, `"debug": true` can be set at the global level. This is `false` by
default.

The following is a much more complex fake example of `/etc/libnss_shim/config.json` - more databases and functions are
defined (but with made-up commands this time), codes are used to pass data at runtime as arguments/environment
variables, `debug` output is enabled, and there are global defaults set for `env` & `workdir` with some
database/function
level overrides:

```
{
  "databases": {
    "group": {
      "functions": {
        "get_all_entries": {
          "command": "command.sh --all"
        },
        "get_entry_by_name": {
          "command": "another_command.sh -n <$name>",
          "workdir": "/different/folder/to/run/in"
        }
      }
    },
    "passwd": {
      "functions": {
        "get_all_entries": {
          "command": "python3 command.py"
        },
        "get_entry_by_uid": {
          "command": "python3 command.py -uid <$uid>"
        },
      },
      "env": {
        "VARIABLE": "overrides_default_for_all_passwd_database_functions",
        "PYTHONDONTWRITEBYTECODE": "1"
      }
    },
    "shadow": {
      "functions": {
        "get_entry_by_name": {
          "command": "/retrieve/shadow",
          "env": {
            "SHADOW_REQ_NAME": "<$name>",
            "VARIABLE": "overrides_default_env_variable_for_this_function"
          }
        }
      }
    }
  },
  "debug": true,
  "env": {
    "SOME_VARIABLE": "set this variable for all databases and their functions"
  },
  "workdir": "/folder/to/execute/in"
}
```

## Commands

Commands can have input arguments passed as environment variables or as arguments using the codes defined in the
Configuration section (such as `<$name>`). To return a response to `libnss_shim`, they can simply print a line
to `stdout` in the comma-separated *nix format common to `/etc/shadow`, `group`, and `passwd`, or alternatively in JSON
form as described below. It is important to note that the NSS `compat` options are not supported (e.g. `+@netgroup`).
Information about the standard Unix [colon-separated form](https://www.debianhelp.co.uk/passwordfile.htm) used
by `group`, `passwd`, etc. on
a [variety of *nix systems](https://www.ibm.com/docs/en/aix/7.2?topic=passwords-using-etcpasswd-file) is available
online.

Although it is best to set all fields explicitly to avoid unexpected issues with default/unset values (nobody wants a
password to be blank for some reason), only the following fields are required in command output:

- `group`
    - `name`
    - `gid`
- `passwd`
    - `name`
    - `uid`
    - `gid`
- `shadow`
    - `name`

If using standard Unix colon-split format, optional fields can be left blank. With JSON, they can be omitted entirely.

If the defined command ran correctly but no results are found for that query, it is expected to exit normally and print
either empty JSON `{}` or nothing at all (other than whitespace/newlines) to `stdout`.

Commands and arguments are split according to POSIX shell syntax, but are not run through a shell, so bash-specific
syntax will not function. For example, a command such as `program1 && program2` will be interpreted as
running `program1` with arguments `&&` and `program2`. Although it is not recommended (see the Security section), it
remains possible to run a shell directly, e.g. `sh -c 'program1 && program2'`.

Here is the expected JSON format from running each database's supported commands, with types indicated. All numbers are
expected in base-10 integer form and must fit within the ranges of the indicated numeric  `int` types:

- `group`
    - `get_all_entries()`

        ```
          {
            "group-name-here": {
              "passwd": (str),
              "gid": (uint32),
              "members": [
                (str),
                (str),
                ...
              ]
            },
            "another-groupname": {
              ...
            }
          }
        ```

    - `get_entry_by_gid(int gid)` - Response should be the same format as `get_all_entries()`, but only a single record

    - `get_entry_by_name(str name)` - Response should be the same format as `get_entry_by_gid(int gid)`

- `passwd`
    - `get_all_entries()`

        ```
          {
            "username-here": {
              "passwd": (str),
              "uid": (uint32),
              "gid": (uint32),
              "gecos": (str),
              "dir": (str),
              "shell": (str)
            },
            "second-username-here": {
              ...
            }
          }
        ```

    - `get_entry_by_uid(int uid)` - Response should be the same format as `get_all_entries()`, but only a single record

    - `get_entry_by_name(str name)` - Response should be the same format as `get_entry_by_uid(int uid)`

- `shadow`
    - `get_all_entries()`

        ```
          {
            "first-username-here": {
              "passwd": (str),
              "last_change": (int64),
              "change_min_days": (int64),
              "change_max_days": (int64),
              "change_warn_days": (int64),
              "change_inactive_days": (int64),
              "expire_date": (int64),
              "reserved": (uint64)
            },
            "username-two": {
              ...
            }
          }
        ```

    - `get_entry_by_name(str name)` - Response should be the same format as `get_all_entries()`, but only a single
      record.
    - *Note*: The final field, `reserved`, is seemingly unused in practice and is typically omitted

## Security

This NSS plugin runs commands defined in the file `/etc/libnss_shim/config.json`, which is only accessible to `root` by
default. Ensure that this file, the commands defined inside it, and any other related resources remain inaccessible to
other users, or the system may be vulnerable to privilege escalation attacks.

It is recommended to pass data (like `<$name>`) using environment variables rather than arguments, except for
testing purposes. Environment variables are generally private, whereas commands/launch args are not.

Commands are not passed through a shell for execution. Although it is possible to run software like `bash`
with `libnss_shim`, using a shell is not recommended as this comes with additional risks such as command injection.

## Useful resources

- NSS Modules Interface
    - The GNU C [library](https://www.gnu.org/software/libc/manual/html_node/NSS-Modules-Interface.html)
- Actions in the NSS configuration
    - The GNU C [library](https://www.gnu.org/software/libc/manual/html_node/Actions-in-the-NSS-configuration.html)
- Rust bindings for `libnss`
    - The `libnss` [crate](https://crates.io/crates/libnss)
- Packaging to `.deb` with Rust
    - The `cargo-deb` [crate](https://crates.io/crates/cargo-deb)
- Example of a `libnss` plugin produced with Rust and packaged as `.deb`
    - The `nss-wiregarden` [crate](https://crates.io/crates/libnss-wiregarden)
- Example of parsing `passwd` and `group` formats with Rust
    - The `parsswd` [crate](https://crates.io/crates/parsswd)
- Testing NSS modules in glibc
    - Geoffrey Thomas's [blog](https://ldpreload.com/blog/testing-glibc-nsswitch)
- NSS - Debathena (useful description of NSS and how it fits into their architecture)
    - MIT Debathena [wiki](https://debathena.mit.edu/trac/wiki/NSS)
- Debathena hacks (links to more NSS-related code for their project)
    - MIT Debathena [website](https://debathena.mit.edu/hacks)
- Debathena NSS module source example
    - MIT Debathena [repository](https://debathena.mit.edu/packages/debathena/libnss-afspag/libnss-afspag-1.0/)
