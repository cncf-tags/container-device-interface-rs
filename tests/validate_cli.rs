use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn validate_bin() -> &'static str {
    env!("CARGO_BIN_EXE_validate")
}

fn manifest_path(path: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

#[test]
fn validate_cli_accepts_good_v1_1_fixture() {
    let output = Command::new(validate_bin())
        .arg(manifest_path("tests/fixtures/cdi-v1.1-good.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        output.status.success(),
        "validate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_accepts_good_fixture_with_explicit_cdi_schema() {
    let output = Command::new(validate_bin())
        .arg("--schema")
        .arg(manifest_path("src/schema/schema.json"))
        .arg(manifest_path("tests/fixtures/cdi-v1.1-good.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        output.status.success(),
        "validate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_accepts_v1_0_legacy_intel_rdt_fields() {
    let output = Command::new(validate_bin())
        .arg(manifest_path(
            "tests/fixtures/cdi-v1.0-legacy-intel-rdt.yaml",
        ))
        .output()
        .expect("run validate binary");

    assert!(
        output.status.success(),
        "validate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_rejects_empty_devices_fixture() {
    let output = Command::new(validate_bin())
        .arg(manifest_path("tests/fixtures/cdi-empty-devices.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        !output.status.success(),
        "validate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_rejects_invalid_annotations() {
    let output = Command::new(validate_bin())
        .arg(manifest_path("tests/fixtures/cdi-invalid-annotation.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        !output.status.success(),
        "validate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_rejects_unknown_fields() {
    let output = Command::new(validate_bin())
        .arg(manifest_path("tests/fixtures/cdi-unknown-field.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        !output.status.success(),
        "validate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_rejects_v1_1_legacy_intel_rdt_fields() {
    let output = Command::new(validate_bin())
        .arg(manifest_path(
            "tests/fixtures/cdi-v1.1-legacy-intel-rdt.yaml",
        ))
        .output()
        .expect("run validate binary");

    assert!(
        !output.status.success(),
        "validate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_rejects_v1_0_enable_monitoring_field() {
    let output = Command::new(validate_bin())
        .arg(manifest_path(
            "tests/fixtures/cdi-v1.0-enable-monitoring.yaml",
        ))
        .output()
        .expect("run validate binary");

    assert!(
        !output.status.success(),
        "validate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_schema_none_skips_validation() {
    let output = Command::new(validate_bin())
        .arg("--schema")
        .arg("none")
        .arg(manifest_path("tests/fixtures/cdi-empty-devices.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        output.status.success(),
        "validate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validate_cli_rejects_cdi_schema_without_defs_json() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let schema_path = tempdir.path().join("schema.json");
    std::fs::copy(manifest_path("src/schema/schema.json"), &schema_path)
        .expect("copy schema without defs");

    let output = Command::new(validate_bin())
        .arg("--schema")
        .arg(&schema_path)
        .arg(manifest_path("tests/fixtures/cdi-v1.1-good.yaml"))
        .output()
        .expect("run validate binary");

    assert!(
        !output.status.success(),
        "validate unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("defs.json"),
        "stderr did not mention defs.json\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
