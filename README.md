# libnss_shim

Respond to [Name Service Switch](https://www.gnu.org/software/libc/manual/html_node/Name-Service-Switch.html) lookups with the output of custom commands. Both JSON and the typical
colon-separated *nix format are supported.

## Overview

`libnss_shim` is an adapter to make integration with NSS easier. It is an NSS/nsswitch service that runs commands
defined per-function in `/etc/libnss_shim/config.json`.
* Commands can output responses to queries either in the colon-delimited Unix
format, or in JSON
* The output of each command execution is parsed from `stdout` and validated before being passed back
to NSS
  * See [Commands](docs/README.md#commands) in the docs for details
* The `group`, `passwd`, and `shadow` NSS databases/services are supported
  * See [Configuration](docs/README.md#configuration) in the docs for details
* See the [documentation](docs) for additional information

## Demonstration

![samples/basic](https://raw.githubusercontent.com/xenago/libnss_shim/main/docs/res/libnss_shim_demo.gif)

A more complex example implementation with config, scripts and `Dockerfile` can be found at
[`samples/advanced`](samples/advanced).

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
  - See [Development](docs/README.md#development) in the docs for information about building for other architectures
- Packaged in `.deb` and `.rpm` formats
  - If those formats are not supported by a target platform, `libnss_shim` might be usable if the `assets` are installed
    as described in `Cargo.toml` prior to running the `debian/postinst` script, but this has not been tested extensively
- To request support for a different configuration, please create an [issue](https://github.com/xenago/libnss_shim/issues)

### Install/Upgrade

1. Prepare the commands/software that will be triggered by `libnss_shim`. See [Commands](docs/README.md#commands) in
   the docs for details.

2. Download the [latest release](https://github.com/xenago/libnss_shim/releases/latest) produced by GitHub Actions.

   **AMD64 deb:**

       curl -sLo libnss_shim.deb https://github.com/xenago/libnss_shim/releases/download/1.2.1/libnss_shim_1.2.1-1_amd64.deb

   **AMD64 RPM:**

       curl -sLo libnss_shim.rpm https://github.com/xenago/libnss_shim/releases/download/1.2.1/libnss_shim-1.2.1-1.x86_64.rpm

   **Full table:**

   | Architecture | Package | Link                                                                                                                               |
   |--------------|---------|------------------------------------------------------------------------------------------------------------------------------------|
   | `amd64`      | `deb`   | [`libnss_shim_1.2.1-1_amd64.deb`](https://github.com/xenago/libnss_shim/releases/download/1.2.1/libnss_shim_1.2.1-1_amd64.deb)      |
   | `amd64`      | `RPM`   | [`libnss_shim-1.2.1-1.x86_64.rpm`](https://github.com/xenago/libnss_shim/releases/download/1.2.1/libnss_shim-1.2.1-1.x86_64.rpm)   |
   | `aarch64`    | `deb`   | [`libnss_shim_1.2.1-1_arm64.deb`](https://github.com/xenago/libnss_shim/releases/download/1.2.1/libnss_shim_1.2.1-1_arm64.deb)     |
   | `aarch64`    | `RPM`   | [`libnss_shim-1.2.1-1.aarch64.rpm`](https://github.com/xenago/libnss_shim/releases/download/1.2.1/libnss_shim-1.2.1-1.aarch64.rpm) |

    See [Downloading](docs/README.md#installation) and [Attestations](docs/README.md#installation) in the docs for more
    details.

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
   defined that output nothing. Updates to the config take effect immediately and can be performed at any time after
   `libnss_shim` has been installed and used, without restarting.

   See [Configuration](docs/README.md#configuration) and [Commands](docs/README.md#commands) in the docs for details.

5. When installed, `libnss_shim` is mapped as `shim` in `/etc/nsswitch.conf` as the last source for all supported
   databases. In that file, the access order for each database's sources can be changed, `shim` can be removed from
   specific locations if not required, etc.
   
   Because `nsswitch.conf` is read only once per-process, any software actively using it will need to be restarted to
   access `libnss_shim` when it is installed. Rebooting the system is often the safest/easiest way to do this:
    ```
    sudo reboot
    ```

   See [Interaction with `/etc/nsswitch.conf`](docs/README.md#interaction-with-etcnsswitchconf) in the docs for details.

6. Perform NSS queries to validate the installation, for example using the built-in `getent` tool.

    Some sample commands to test your implementation:
    ```
    getent group
    getent passwd
    getent shadow
    getent group <groupname>
    ```
    A very basic test config is available that will respond to `getent group` calls with a fake group (like in the
    [demo GIF](#demonstration)):
    
       curl -sLo /etc/libnss_shim/config.json https://raw.githubusercontent.com/xenago/libnss_shim/main/samples/basic/custom_config.json
       getent group | tail -1
    
    If the installation worked, the output should look like:
    
       test-shim-group::1008:fake-username,another-user

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

## Security

The `libnss_shim` NSS plugin runs commands defined in `/etc/libnss_shim/config.json`, which only `root` can edit by
default. Ensure that this file, the commands defined inside it, and any other related resources remain read-only to
other users, or the system may be vulnerable to privilege escalation attacks. Do not store secrets in `config.json` or
any other file which can be read by non-`root` users.

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

See [SECURITY.md](docs/SECURITY.md) for information about reporting security problems.
