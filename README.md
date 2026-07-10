# Container Device Interface (Rust)

Rust implementation of the
[Container Device Interface](https://github.com/cncf-tags/container-device-interface)
(CDI) specification, at parity with CDI v1.1.0.

CDI lets container runtimes support third-party devices (GPUs, FPGAs, and
other accelerators) through vendor-provided JSON/YAML specs instead of
runtime-specific plugins.

## Library

```bash
cargo add container-device-interface
```

The API mirrors the Go implementation: CDI specs are discovered from the
standard spec directories and requested devices are injected into an OCI
runtime spec:

```rust
use container_device_interface::default_cache;
use oci_spec::runtime::Spec;

let mut oci_spec = Spec::default();
default_cache::inject_devices(&mut oci_spec, vec!["vendor.com/device=gpu0".into()])?;
```

Full API documentation: <https://docs.rs/container-device-interface>

## Binaries and signed artifacts

Each release ships the `cdi` and `validate` CLI tools and the
`libcontainer_device_interface.so` cdylib for x86_64 and aarch64 as
reproducible tarballs with cosign signatures, an SPDX SBOM, and SLSA
provenance. Artifacts and verification material are on the
[releases page](https://github.com/cncf-tags/container-device-interface-rs/releases).

## Building from source

See [BUILDING.md](BUILDING.md) for local builds and for
reproducing release artifacts bit-for-bit.

## License

Apache-2.0
