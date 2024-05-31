#!/bin/sh
set -e

cd /libnss_shim

echo "BUILD: Setting version to $LIBNSS_SHIM_VERSION"
sed -i "s/0.0.0/$LIBNSS_SHIM_VERSION/g" Cargo.toml

echo "BUILD: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"

echo "BUILD: Building for release"
cargo build --release --verbose
echo "BUILD: built for release"
ls -lah target/release

echo "BUILD: Packaging deb"
cargo install --version 1.44.1 cargo-deb
cargo deb --verbose
echo "BUILD: deb packaged"
ls -lah target/debian

echo "BUILD: Packaging RPM"
cargo install --version 0.14.0 cargo-generate-rpm
strip -s target/release/libnss_shim.so
cargo generate-rpm --payload-compress none
echo "BUILD: RPM packaged"
ls -lah target/generate-rpm
