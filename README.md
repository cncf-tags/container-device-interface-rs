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

### Spec validation

Loading a Spec always applies strict parsing (unknown fields are rejected)
and the same structural checks as the Go implementation: version gates,
vendor and class names, annotations, container edits and devices.
JSON-schema validation is a separate, opt-in step, like Go's
`cdi.SetSpecValidator`: no validator is installed by default and no schema
code is compiled in. To check every loaded Spec against the embedded CDI
schema, enable the `schema-validation` feature and install one:

```rust
use container_device_interface::{schema::SchemaValidator, spec::set_spec_validator};

set_spec_validator(SchemaValidator::builtin());
```

Any `Send + Sync` closure `Fn(&Spec) -> anyhow::Result<()>` can be installed
the same way. The `cdi` CLI validates against the built-in schema unless
started with `--schema none`.

### Cargo features

| Feature             | Default | Adds                                                                        |
| ------------------- | ------- | --------------------------------------------------------------------------- |
| `schema-validation` | off     | `schema` module and `SchemaValidator`; pulls in `jsonschema`                |
| `cli`               | off     | `cdi` and `validate` binaries; pulls in `clap`, implies `schema-validation` |

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
