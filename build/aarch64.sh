#!/bin/sh
set -e

cd /libnss_shim

# Uncomment this to set a custom version number
# This is normally done at the start before doing any builds in CI
#echo "BUILD ARM64: Setting version to $LIBNSS_SHIM_VERSION"
#sed -i "s/0.0.0/$LIBNSS_SHIM_VERSION/g" Cargo.toml

# Uncomment this to install necessary dependencies
# This is normally done in the container build step in CI
#echo "================"
#echo "BUILD ARM64: Installing Rust"
#curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
#. "$HOME/.cargo/env"
#echo "================"
#echo "BUILD ARM64: Installing cargo dependencies"
#cargo install --version 2.2.0 cargo-deb
#cargo install --version 0.14.0 cargo-generate-rpm

echo "================"
echo "BUILD ARM64: Building for release"
cargo build --release --verbose
echo "BUILD ARM64: Built for release"
ls -lah target/release

echo "================"
echo "BUILD ARM64: Packaging deb"
cargo deb --verbose --no-build
echo "BUILD ARM64: deb packaged"
ls -lah target/debian

echo "================"
echo "BUILD ARM64: Packaging RPM"
cargo generate-rpm --payload-compress none
echo "BUILD ARM64: RPM packaged"
ls -lah target/generate-rpm

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
