# Documentation

Additional documentation relevant to use of `libnss_shim`.

### Downloading

It is also possible to get the latest release URLs using the rate-limited public GitHub API:

       curl -s https://api.github.com/repos/xenago/libnss_shim/releases/latest | grep "browser_download_url.*.$" | cut -d : -f 2,3 | tr -d \" | tr -d " "

By filtering those results further and adding a final `curl` download, the latest version of a specific package can
be acquired in one line. For instance, this command will download the latest 64-bit x86 `deb` installer:

       curl -s https://api.github.com/repos/xenago/libnss_shim/releases/latest | grep "browser_download_url.*.$" | cut -d : -f 2,3 | tr -d \" | tr -d " " | grep amd64.deb | xargs -n 1 curl -sLO

## Attestations

Validating downloaded packages before installation can be done with
[artifact attestations](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds).
With the use of artifact attestations, it is possible to check the hashes of downloaded packages to establish build
provenance and confirm details like the commit SHA, pipeline trigger, etc. Validation is recommended as it can prevent
issues arising from corrupt files or malicious actors sharing modified builds.

Artifact attestations are available for `libnss_shim` releases as of version `1.2.1`. To verify artifact attestations
for official packages built with GitHub Actions, the
[GitHub CLI](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds#verifying-artifact-attestations-with-the-github-cli)
can be used. Example command:

    gh attestation verify /path/to/libnss_shim.deb -R xenago/libnss_shim

The GitHub CLI requires using a GitHub account,
but it is also possible to [view attestations online](https://github.com/xenago/libnss_shim/attestations) without an
API key.

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

Using only that information, here is the
[extremely basic test example of `config.json`](samples/basic/custom_config.json) - one database is defined,
`group`, with just a single function, `get_all_entries`:

```
{
  "databases": {
    "group": {
      "functions": {
        "get_all_entries": {
          "command": "echo 'test-shim-group::1008:fake-username,another-user'"
        }
      }
    }
  }
}
```

The command defined for `get_all_entries` prints out a single line to `stdout`, describing a fake group
called `test-shim-group` with `gid=1008` and two members. That output is then captured by `libss_shim` and returned
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
[Configuration section](#configuration) (such as `<$name>`). To return a response to `libnss_shim`, they can simply
print a line to `stdout` in the comma-separated *nix format common to `/etc/shadow`, `group`, and `passwd`, or
alternatively in JSON form as described below. It is important to note that the NSS `compat` options are not supported
(e.g. `+@netgroup`). Information about the [colon-separated format](https://www.debianhelp.co.uk/passwordfile.htm) used for `group`, `passwd`, etc. on a
[variety of *nix systems](https://www.ibm.com/docs/en/aix/7.2?topic=passwords-using-etcpasswd-file) is available online.

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
running `program1` with arguments `&&` and `program2`. Although it is not recommended (see the
[Security section](#security)), it remains possible to run a shell directly, e.g. `sh -c 'program1 && program2'`.

Here is the expected JSON format from running each database's supported commands, with types indicated. All numbers are
expected in base-10 integer form and must fit within the ranges of the indicated numeric  `int` types (`isize`
and `usize` are platform-dependent and can be 32 or 64-bits):

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

   - `get_entry_by_gid(uint32 gid)` - Response should be the same format as `get_all_entries()`, but only a single
     record

   - `get_entry_by_name(str name)` - Response should be the same format as `get_entry_by_gid(uint32 gid)`

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

   - `get_entry_by_uid(uint32 uid)` - Response should be the same format as `get_all_entries()`, but only a single
     record

   - `get_entry_by_name(str name)` - Response should be the same format as `get_entry_by_uid(uint32 uid)`

- `shadow`
   - `get_all_entries()`

       ```
         {
           "first-username-here": {
             "passwd": (str),
             "last_change": (isize),
             "change_min_days": (isize),
             "change_max_days": (isize),
             "change_warn_days": (isize),
             "change_inactive_days": (isize),
             "expire_date": (isize),
             "reserved": (usize)
           },
           "username-two": {
             ...
           }
         }
       ```

   - `get_entry_by_name(str name)` - Response should be the same format as `get_all_entries()`, but only a single
     record
   - *Note*: The final field, `reserved`, is seemingly unused in practice and is typically omitted

### Interaction with `/etc/nsswitch.conf`

Originally from [issue #5](https://github.com/xenago/libnss_shim/issues/5#issuecomment-2151243010).

> When implementing `get_entry_by_gid` do we need to return a value for preexisting gid in `/etc/group` file? Also,
> same question regarding `get_entry_by_name` for `group` and `passwd`.

No need to return entries if that same content is already present on disk in e.g. `/etc/group`/`etc/passwd`. That is
taken care of by the existing mappings as set up by the default installation process.

From [`README.md`](../README.md#installupgrade):

> libnss_shim is mapped as shim in /etc/nsswitch.conf as the last source for all supported databases

What this looks like in `/etc/nsswitch.conf` by default after installing `libnss_shim` on a clean AlmaLinux system:

    (...)
    passwd:     sss files systemd shim
    shadow:     files shim
    group:      sss files systemd shim
    (...)

This means that by default the `shim` will be called only after lookups to the other databases listed before it, and
only if the others failed. So if there is already an entry on disk (or in any preceding database) for that particular
query then `libnss_shim` will never be called.

You can demonstrate this effect by configuring `libnss_shim` to respond with different information than is on disk, and
running these queries:

    getent group root
    getent -s files group root
    getent -s shim group root

![image](https://raw.githubusercontent.com/xenago/libnss_shim/main/docs/res/nsswitch_order_demo_1.png)

The first one makes a normal request which follows the priority order for`group` set in `/etc/nsswitch.conf`: `sss`,
`files`, `systemd`, `shim`. Because `sss` responds with nothing, NSS moves on and requests it from `files`, which
succeeds so `systemd`/`shim` are never queried.

The second command specifically requests it from `files` (i.e. `/etc/group/`), so the same information as the first
command is returned.

The third command requests it from `shim`, so `libnss_shim` is used and the custom entry value is returned.

If the order of `group` in `/etc/nsswitch.conf` is changed so that `shim` is first, then `getent group root` will
return the custom value right away and never check the other databases:

![image](https://raw.githubusercontent.com/xenago/libnss_shim/main/docs/res/nsswitch_order_demo_2.png)

## Development

Builds can be run inside temporary containers, like in [CI](../.github/workflows/ci.yaml):

1. Ensure Docker is installed and the `docker` command is available to the running user

2. Ensure the `libnss_shim` repository has been cloned:

       git clone https://github.com/xenago/libnss_shim.git

3. Run the build script inside a container using the `docker run` command

    * Replace `/path/to/cloned/libnss_shim` with the actual location of the cloned repository
    * Edit `--version_number=0.0.0` if a specific version number is desired (SemVer format)

          docker run -v /path/to/cloned/libnss_shim:/libnss_shim --rm -it quay.io/pypa/manylinux2014_x86_64:latest bash -c 'cd /libnss_shim && bash build.sh --version_number=0.0.0'

   Note: it may be possible to build `libnss_shim` for additional architectures and operating systems by using different
   [`manylinux`](https://github.com/pypa/manylinux) versions or other containers in combination with QEMU.
   [`cross`](https://github.com/cross-rs/cross) might also help in this regard.

4. The build script will output packages in the following subdirectories of the cloned repo:

    * `target/debian/*.deb`
    * `target/generate-rpm/*.rpm`

### Useful resources

See [ACKNOWLEDGMENTS.md](ACKNOWLEDGMENTS.md)
