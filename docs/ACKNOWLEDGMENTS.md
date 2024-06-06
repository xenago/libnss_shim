# Resources

Sources relevant to the development and use of `libnss_shim`.

---

#### *Python wheels that work on any linux (almost)*
- Multi-arch linux containers with wide compatibility for build needs
- The `manylinux` project [repository](https://github.com/pypa/manylinux)

#### *Run-On-Arch GitHub Action*
- Run containers on various CPU architectures with QEMU
- The `run-on-arch-action` [repository](https://github.com/uraimo/run-on-arch-action)

#### *Rust bindings for creating libnss modules*
- The `libnss` [crate](https://crates.io/crates/libnss)

#### *Debian packages from Cargo projects*
- The `cargo-deb` [crate](https://crates.io/crates/cargo-deb)

#### *Generate a binary RPM package (.rpm) from Cargo projects*
- The `cargo-generate-rpm` [crate](https://crates.io/crates/cargo-generate-rpm)

#### *Building Rust binaries in CI that work with older GLIBC*
- Jakub Beránek, AKA Kobzol's [blog](https://kobzol.github.io/rust/ci/2021/05/07/building-rust-binaries-in-ci-that-work-with-older-glibc.html)

#### *NSS Modules Interface*
- The GNU C [library](https://www.gnu.org/software/libc/manual/html_node/NSS-Modules-Interface.html)

#### *Actions in the NSS configuration*
- The GNU C [library](https://www.gnu.org/software/libc/manual/html_node/Actions-in-the-NSS-configuration.html)

#### *Testing NSS modules in glibc*
- Geoffrey Thomas's [blog](https://ldpreload.com/blog/testing-glibc-nsswitch)

#### *NSS - Debathena*
- A useful description of NSS and how it fits into the Debathena architecture
- MIT Debathena [wiki](https://debathena.mit.edu/trac/wiki/NSS)

#### *Debathena hacks*
- Links to more NSS-related code for the Debathena project
- MIT Debathena [website](https://debathena.mit.edu/hacks)

#### Debathena NSS module source example
- MIT Debathena [repository](https://debathena.mit.edu/packages/debathena/libnss-afspag/libnss-afspag-1.0/)

#### Example of a `libnss` plugin produced with Rust and packaged as `.deb`
- The `nss-wiregarden` [crate](https://crates.io/crates/libnss-wiregarden)

#### Example of parsing `passwd` and `group` formats with Rust
- The `parsswd` [crate](https://crates.io/crates/parsswd)
