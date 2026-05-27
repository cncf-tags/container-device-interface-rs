# Repository Guidelines

## Project Structure & Module Organization

This is the Rust implementation of the Container Device Interface. Library code
lives under `src/`, with the public module surface declared in `src/lib.rs`.
Core CDI behavior is split across `src/cache.rs`, `src/device.rs`,
`src/spec.rs`, `src/parser.rs`, and `src/container_edits.rs`. Versioned spec
types live in `src/specs/`, generated-spec helpers are in `src/generate/`, and
the built-in JSON schema lives in `src/schema/schema.json` plus
`src/schema/defs.json`. CLI entry points are `src/bin/cdi.rs` and
`src/bin/validate.rs`; integration tests and CDI fixture YAMLs are under
`tests/`.

## Build, Test, and Formatting Commands

- `cargo build --all-targets`: build the library and both binaries.
- `cargo test`: run unit and integration tests.
- `cargo test --test validate_cli`: focus on validator CLI/schema behavior.
- `cargo fmt --all -- --check`: check formatting; use `cargo fmt --all` to fix.
- `cargo clippy --all-targets --all-features -- -D warnings`: match CI linting.

## Schema Sync With The Main CDI Repo

The canonical schema source is the upstream CDI repo, located at:
`https://github.com/cncf-tags/container-device-interface.git`.

Copy `schema.json` and `defs.json` together; `src/schema/mod.rs` rewrites
`defs.json#/definitions/...` references into inline definitions when compiling
the built-in schema. After syncing, reconcile any spec-type or version-gate
changes in `src/specs/config.rs`, `src/specs/oci.rs`, and `src/version.rs`, then
run `cargo test --test validate_cli` and `cargo test schema`.

## Coding Style & Contribution Notes

Use standard Rust 2021 style and keep serde field names aligned with the CDI
schema. Prefer typed serde parsing and validation helpers over ad hoc JSON/YAML
string handling. For schema-facing changes, add or update fixtures in
`tests/fixtures/` and keep CLI coverage in `tests/validate_cli.rs`. Follow
`CONTRIBUTING.md` for review norms and sign commits with `git commit -s`.
