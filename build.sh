#!/bin/bash

# Exit immediately on error
set -e

# Set defaults
build_libnss_shim=true
install_rust=true
install_cargo_deps=true
package_deb=true
package_rpm=true
version_number=0.0.0

# Override default settings based on launch arguments
# Idea from https://www.drupal.org/node/244924#script-based-on-guidelines-given-above
while [ $# -gt 0 ]; do
  case "$1" in
    --build_libnss_shim=* )
      build_libnss_shim="${1#*=}"
      ;;
    --install_cargo_deps=* )
      install_cargo_deps="${1#*=}"
      ;;
    --install_rust=* )
      install_rust="${1#*=}"
      ;;
    --package_deb=* )
      package_deb="${1#*=}"
      ;;
    --package_rpm=* )
      package_rpm="${1#*=}"
      ;;
    --version_number=* )
      version_number="${1#*=}"
      ;;
    * )
      printf "BUILD ERROR: unknown argument %s\n" ${!$1}
      exit 1
  esac
  shift
done

# Determine platform and warn if it is unsupported
echo "BUILD: Detecting architecture"
architecture=$(uname -m)
case $architecture in
  x86_64 | amd64 )
    architecture="amd64"
    ;;
  arm | arm64 | aarch64 )
    architecture="aarch64"
    ;;
  * )
    echo "BUILD WARNING: Unexpected architecture ${architecture}"
esac
echo "BUILD ${architecture}: Architecture detected"
echo "================"

# Prep
if [[ $version_number != "0.0.0" ]]
  then
    echo "BUILD ${architecture}: Setting version to ${version_number}"
    sed -i "0,/^version /s/=.*$/= \"${version_number}\"/" Cargo.toml
    echo "================"
fi
if $install_rust
  then
    echo "BUILD ${architecture}: Installing Rust"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
    echo "================"
fi
if $install_cargo_deps
  then
    echo "BUILD ${architecture}: Installing cargo dependencies"
    cargo install --version 2.2.0 cargo-deb
    cargo install --version 0.14.0 cargo-generate-rpm
    echo "================"
fi

# Build
if $build_libnss_shim
  then
    echo "BUILD ${architecture}: Building for release"
    cargo build --release --verbose
    echo "BUILD ${architecture}: Built for release"
    ls -lah target/release
    echo "================"
fi

# Packaging
if $package_deb
  then
    echo "BUILD ${architecture}: Packaging deb"
    cargo deb --verbose --no-build
    echo "BUILD ${architecture}: Packaged deb"
    ls -lah target/debian
    echo "================"
fi
if $package_rpm
  then
    echo "BUILD ${architecture}: Packaging RPM"
    cargo generate-rpm --payload-compress none
    echo "BUILD ${architecture}: Packaged RPM"
    ls -lah target/generate-rpm
    echo "================"
fi

# TODO: enable local cross-build to arm64 with docker-in-docker using `cross`
#env CROSS_CONTAINER_IN_CONTAINER=true cross build --release --target aarch64-unknown-linux-gnu

#echo "BUILD ARM64: Cross-building for release (arm64/aarch64)"
#cargo install --version 0.2.5 cross
#env CROSS_CONTAINER_IN_CONTAINER=true cross build --release --target aarch64-unknown-linux-gnu
#echo "BUILD ARM64: built for release (arm64/aarch64)"
#ls -lah target/aarch64-unknown-linux-gnu/release
#
#echo "BUILD ARM64: Packaging deb (arm64/aarch64)"
#cargo deb --target=aarch64-unknown-linux-gnu --no-build --verbose
#echo "BUILD ARM64: deb packaged (arm64/aarch64)"
#ls -lah target/aarch64-unknown-linux-gnu/debian
#
#echo "BUILD ARM64: Packaging RPM (arm64/aarch64)"
## FIXME: use script when possible for --auto-req
## See https://github.com/cat-in-136/cargo-generate-rpm/issues/107
## --auto-req "cross/find-requires.sh"
#cargo generate-rpm --payload-compress none --target aarch64-unknown-linux-gnu --auto-req disabled
#echo "BUILD ARM64: RPM packaged (arm64/aarch64)"
#ls -lah target/aarch64-unknown-linux-gnu/generate-rpm
