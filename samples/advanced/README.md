# Advanced Sample

Build and run the sample container image; useful as an example or to test `libnss_shim` functionality.

## Details

As defined in the `Dockerfile`, three Python scripts are installed in the container image and configured for use by
`libnss_shim`:

* `custom-group-resolver.py`
* `custom-passwd-resolver.py`
* `custom-shadow-resolver.py`

Each script services requests for a specific NSS database. As set in `custom_config.json`, environment variables are
used to pass data to the scripts.

## Usage

Docker is required to proceed. Podman may also work, but this has not been tested extensively.

1. After cloning the repository, build the sample image as `libnss_shim_container` & run it:

       cd libnss_shim/samples/advanced
       docker build -t libnss_shim_container . && docker run --rm -it libnss_shim_container

2. Once in the container shell, make requests to test `libnss_shim`:

       getent group test-shim-users
       getent passwd | grep test-shim-user
       getent shadow test-shim-user-3

3. When finished, quit the container shell and delete the image:

       exit
       docker image rm libnss_shim_container
