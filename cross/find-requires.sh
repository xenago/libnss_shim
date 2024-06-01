#!/bin/sh

# Fail fast on any error
set -e

# This is designed to replicate the output from the `find-requires` RPM script
# http://ftp.rpm.org/max-rpm/s1-rpm-depend-auto-depend.html
# That script uses `ldd`, which doesn't seem to work with cross-compiled binaries
# https://github.com/cat-in-136/cargo-generate-rpm/issues/107
objdump -x "$1" | grep NEEDED  | awk '{print $NF}'
