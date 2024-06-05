# libnss_shim

Respond to [Name Service Switch](https://www.gnu.org/software/libc/manual/html_node/Name-Service-Switch.html) lookups with the output of custom commands. Both JSON and the typical
colon-separated *nix format are supported.

## Overview

`libnss_shim` is an adapter to make integration with NSS easier. It is an NSS/nsswitch service that runs commands
defined per-function in `config.json`. Commands can output responses to queries either in the colon-delimited Unix
format, or in JSON. The output of each command execution is parsed from `stdout` and validated before being passed back
to NSS (see the [Commands section](#commands)). The `group`, `passwd`, and `shadow` NSS databases/services are supported
(see the [Configuration section](#configuration))

## Demonstration

![libnss_shim GIF demo](https://raw.githubusercontent.com/xenago/libnss_shim/main/doc/libnss_shim_demo.gif)

## Background

Custom [PAM](https://www.man7.org/linux/man-pages/man8/pam.8.html) modules alone are not enough to create a custom Linux authentication process - integration with NSS is
also required to inject custom user data to `group`/`passwd`/`shadow` lookups earlier in the login flow.

In other words: NSS determines if an account exists, and PAM determines how an account can be accessed.

For example, [SSSD](https://sssd.io) leverages both NSS and PAM to enable seamless LDAP authentication. Integrating directly with
NSS can be difficult, so `libnss_shim` was created to allow any command that can print to `stdout` in a supported format
to be used with NSS.

## Installation

### Compatibility notes

- Tested on:
  - Debian 11
  - Debian 12
  - Ubuntu 20.04
  - Ubuntu 22.04
  - Ubuntu 24.04
  - CentOS 7
  - AlmaLinux 8
  - AlmaLinux 9
- Builds for `amd64` and `aarch64` architectures
  - See the [Development section](#development) for information about building for other architectures
- Packaged in `.deb` and `.rpm` formats
  - If those formats are not supported by a target platform, `libnss_shim` might be usable if the `assets` are installed
    as described in `Cargo.toml` prior to running the `debian/postinst` script, but this has not been tested extensively
- To request support for a different configuration, please create a GitHub Issue

### Install/Upgrade

1. Prepare the commands/software that will be triggered by `libnss_shim` (see the [Commands section](#commands)).

2. Download the latest release produced by GitHub Actions.

   **AMD64 deb:**
    ```
    curl -sLo libnss_shim.deb https://github.com/xenago/libnss_shim/releases/download/1.2.0/libnss_shim_1.2.0_amd64.deb
    ```
   **AMD64 RPM:**
    ```
    curl -sLo libnss_shim.rpm https://github.com/xenago/libnss_shim/releases/download/1.2.0/libnss_shim-1.2.0-1_x86_64.rpm
    ```
   **Full table:**

   | Architecture | Package | Link                                                                                                                               |
   |--------------|---------|------------------------------------------------------------------------------------------------------------------------------------|
   | `amd64`      | `deb`   | [`libnss_shim_1.2.0_amd64.deb`](https://github.com/xenago/libnss_shim/releases/download/1.2.0/libnss_shim_1.2.0_amd64.deb)         |
   | `amd64`      | `RPM`   | [`libnss_shim-1.2.0-1.x86_64.rpm`](https://github.com/xenago/libnss_shim/releases/download/1.2.0/libnss_shim-1.2.0-1.x86_64.rpm)   |
   | `aarch64`    | `deb`   | [`libnss_shim_1.2.0_arm64.deb`](https://github.com/xenago/libnss_shim/releases/download/1.2.0/libnss_shim_1.2.0_arm64.deb)         |
   | `aarch64`    | `RPM`   | [`libnss_shim-1.2.0-1.aarch64.rpm`](https://github.com/xenago/libnss_shim/releases/download/1.2.0/libnss_shim-1.2.0-1.aarch64.rpm) |

3. Install or upgrade it directly with `dpkg` or `rpm`.

   **deb:**
    ```
    sudo dpkg -i libnss_shim.deb
    ```
   **RPM:**
    ```
    sudo rpm -Uv ./libnss_shim.rpm
    ```

4. Configure the shim by importing a custom `config.json`.

   e.g.
    ```
    sudo cp custom_config.json /etc/libnss_shim/config.json
    ```
   Using the default `config.json`, `libnss_shim` should have no effect, as the default configuration has commands
   defined that output nothing (see the [Configuration section](#configuration) for details). Updates to the config
   take effect immediately and can be performed at any time after `libnss_shim` has been installed and used, without
   restarting.

5. When installed, `libnss_shim` is mapped as `shim` in `/etc/nsswitch.conf` as the last source for all supported
   databases. In that file, the access order for each database's sources can be changed, `shim` can be removed from
   specific locations if not required, etc. Because `nsswitch.conf` is read only once per-process, any software actively
   using it will need to be started or restarted when changes are made.

   Rebooting the system is often the safest/easiest way to do this:
    ```
    sudo reboot
    ```

6. Perform NSS queries to validate the installation, for example using the built-in `getent` tool.

    Some sample commands to test your implementation:
    ```
    getent group
    getent passwd
    getent shadow
    getent group <groupname>
    ```
    A very basic test config is available that will respond to `getent group` calls with a fake group (see the demo GIF
    at the top of this file):
    
       curl -sLo /etc/libnss_shim/config.json https://raw.githubusercontent.com/xenago/libnss_shim/main/samples/basic/custom_config.json
       getent group | tail -1
    
    If the installation worked, the output should look like:
    
       test-shim-group::1008:fake-username,another-user
    
    A more complex configuration example can be found at [`samples/advanced`](samples/advanced), with a `Dockerfile`.

## Uninstall

1. To remove `libnss_shim`, run the same package manager used for installation.

   **deb:**
    ```
    sudo dpkg -r libnss_shim
    ```
   **RPM:**
    ```
    sudo rpm -e libnss_shim
    ```

2. If removed, restarting affected applications is required. A system reboot is an effective way to do this.

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

## Security

This NSS plugin runs commands defined in the file `/etc/libnss_shim/config.json`, which is only writable by the `root` 
user by default. Ensure that this file, the commands defined inside it, and any other related resources remain read-only
to other users, or the system may be vulnerable to privilege escalation attacks. Do not store secrets in `config.json`
or any other file which can be read by non-`root` users.

To enable non-root users to access resources defined by `libnss_shim`, they must be able to access the commands defined
in `config.json`. For example, if a file `group-script.py` is being used to resolve `group` queries, it will need to be
readable (along with the Python interpreter used to run it):

    sudo chown root:root /path/to/custom/group-script.py
    sudo chmod 644 /path/to/custom/group-script.py

However, as the `shadow` database is generally only accessed via `su`/`setuid` etc., programs used to resolve `shadow`
queries can be left as `640`:

    sudo chown root:root /path/to/custom/shadow-script.py
    sudo chmod 640 /path/to/custom/shadow-script.py

It is recommended to pass data (like `<$name>`) using environment variables rather than arguments, except for
testing purposes. Environment variables are generally private, whereas commands/launch args are not.

Commands are not passed through a shell for execution. Although it is possible to run software like `bash`
with `libnss_shim`, using a shell is not recommended as this comes with additional risks such as command injection.

To verify artifact attestations for official releases build with GitHub Actions, the [GitHub CLI](https://docs.github.com/en/actions/security-guides/using-artifact-attestations-to-establish-provenance-for-builds#verifying-artifact-attestations-with-the-github-cli)
can be used. Note that this is not available for versions `<=1.2.0`. Example command:

    gh attestation verify /set/the/path/to/libnss_shim.deb -R xenago/libnss_shim

Please report problems by creating GitHub Issues or [private advisories](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability).

## Development

Builds can be run inside temporary containers:

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

## Useful resources

- *Python wheels that work on any linux (almost)*
  - Multi-arch linux containers with wide compatibility for build needs
  - The `manylinux` project [repository](https://github.com/pypa/manylinux)
- *Run-On-Arch GitHub Action*
  - Run containers on various CPU architectures with QEMU
  - The `run-on-arch-action` [repository](https://github.com/uraimo/run-on-arch-action)
- *Rust bindings for creating libnss modules*
  - The `libnss` [crate](https://crates.io/crates/libnss)
- *Debian packages from Cargo projects*
  - The `cargo-deb` [crate](https://crates.io/crates/cargo-deb)
- *Generate a binary RPM package (.rpm) from Cargo projects*
  - The `cargo-generate-rpm` [crate](https://crates.io/crates/cargo-generate-rpm)
- *Building Rust binaries in CI that work with older GLIBC*
    - Jakub Beránek, AKA Kobzol's [blog](https://kobzol.github.io/rust/ci/2021/05/07/building-rust-binaries-in-ci-that-work-with-older-glibc.html)
- *NSS Modules Interface*
    - The GNU C [library](https://www.gnu.org/software/libc/manual/html_node/NSS-Modules-Interface.html)
- *Actions in the NSS configuration*
    - The GNU C [library](https://www.gnu.org/software/libc/manual/html_node/Actions-in-the-NSS-configuration.html)
- *Testing NSS modules in glibc*
  - Geoffrey Thomas's [blog](https://ldpreload.com/blog/testing-glibc-nsswitch)
- *NSS - Debathena*
  - A useful description of NSS and how it fits into the Debathena architecture
  - MIT Debathena [wiki](https://debathena.mit.edu/trac/wiki/NSS)
- *Debathena hacks*
  - Links to more NSS-related code for the Debathena project
  - MIT Debathena [website](https://debathena.mit.edu/hacks)
- Debathena NSS module source example
  - MIT Debathena [repository](https://debathena.mit.edu/packages/debathena/libnss-afspag/libnss-afspag-1.0/)
- Example of a `libnss` plugin produced with Rust and packaged as `.deb`
    - The `nss-wiregarden` [crate](https://crates.io/crates/libnss-wiregarden)
- Example of parsing `passwd` and `group` formats with Rust
    - The `parsswd` [crate](https://crates.io/crates/parsswd)
