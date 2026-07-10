# Building and reproducing releases

## Build

The compiler version is pinned in `rust-toolchain.toml`; rustup installs it
on first use.

```bash
cargo build --release --locked
```

Release artifacts (`cdi`, `validate`, `libcontainer_device_interface.so`)
are built with `./scripts/build-release.sh <target>`, which additionally
remaps machine-specific paths so the artifacts are byte-reproducible. The
`Reproducible build` workflow enforces this on every pull request by
building twice from different locations and requiring identical hashes.

Codegen policy (size optimization, LTO, `panic=abort`) lives in
`[profile.release]` in `Cargo.toml`.

## Reproduce a release build

Signature checks prove *who* built an artifact; a rebuild proves *what* was
built. Building a release tag yourself must yield exactly the published
sha256:

```bash
git clone --branch "$TAG" --depth 1 \
  https://github.com/cncf-tags/container-device-interface-rs
cd container-device-interface-rs
./scripts/build-release.sh "$TARGET"
sha256sum "target/${TARGET}/release/cdi" \
  "target/${TARGET}/release/validate" \
  "target/${TARGET}/release/libcontainer_device_interface.so"
```

A mismatch means your toolchain deviates from `rust-toolchain.toml` (check
`rustc -V`) or the artifact was not built from that tag.
