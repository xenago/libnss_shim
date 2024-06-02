#!/bin/sh
set -e

cd /libnss_shim

# Uncomment this to set a custom version number
# This is normally done at the start before doing any builds in CI
#echo "BUILD AMD64: Setting version to $LIBNSS_SHIM_VERSION"
#sed -i "s/0.0.0/$LIBNSS_SHIM_VERSION/g" Cargo.toml

# Uncomment this to install necessary dependencies
# This is normally done in the container build step in CI
#echo "================"
#echo "BUILD AMD64: Installing Rust"
#curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
#. "$HOME/.cargo/env"
#echo "================"
#echo "BUILD AMD64: Installing cargo dependencies"
#cargo install --version 2.2.0 cargo-deb
#cargo install --version 0.14.0 cargo-generate-rpm

echo "================"
echo "BUILD AMD64: Building for release"
cargo build --release --verbose
echo "BUILD AMD64: Built for release"
ls -lah target/release

echo "================"
echo "BUILD AMD64: Packaging deb"
cargo deb --verbose --no-build
echo "BUILD AMD64: deb packaged"
ls -lah target/debian

echo "================"
echo "BUILD AMD64: Packaging RPM"
strip -s target/release/libnss_shim.so
cargo generate-rpm --payload-compress none
echo "BUILD AMD64: RPM packaged"
ls -lah target/generate-rpm
