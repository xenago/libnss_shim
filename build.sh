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
cargo deb --verbose --no-build
echo "BUILD: deb packaged"
ls -lah target/debian

echo "BUILD: Packaging RPM"
cargo install --version 0.14.0 cargo-generate-rpm
strip -s target/release/libnss_shim.so
cargo generate-rpm --payload-compress none
echo "BUILD: RPM packaged"
ls -lah target/generate-rpm

# TODO: enable local cross-build with docker-in-docker

#echo "BUILD: Cross-building for release (arm64/aarch64)"
#cargo install --version 0.2.5 cross
#env CROSS_CONTAINER_IN_CONTAINER=true cross build --release --target aarch64-unknown-linux-gnu
#echo "BUILD: built for release (arm64/aarch64)"
#ls -lah target/aarch64-unknown-linux-gnu/release
#
#echo "BUILD: Packaging deb (arm64/aarch64)"
#cargo deb --target=aarch64-unknown-linux-gnu --no-build --verbose
#echo "BUILD: deb packaged (arm64/aarch64)"
#ls -lah target/aarch64-unknown-linux-gnu/debian
#
#echo "BUILD: Packaging RPM (arm64/aarch64)"
## FIXME: use script when possible for --auto-req
## See https://github.com/cat-in-136/cargo-generate-rpm/issues/107
## --auto-req "cross/find-requires.sh"
#cargo generate-rpm --payload-compress none --target aarch64-unknown-linux-gnu --auto-req disabled
#echo "BUILD: RPM packaged (arm64/aarch64)"
#ls -lah target/aarch64-unknown-linux-gnu/generate-rpm
