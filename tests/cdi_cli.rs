// Spawns the cdi binary; miri cannot emulate process creation.
#![cfg(not(miri))]

use std::{fs, process::Command};

fn cdi_bin() -> &'static str {
    env!("CARGO_BIN_EXE_cdi")
}

#[test]
fn cdi_cli_help_succeeds() {
    let output = Command::new(cdi_bin()).arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("devices"));
    assert!(stdout.contains("inject"));
}

#[test]
fn cdi_cli_lists_devices() {
    let output = Command::new(cdi_bin()).arg("devices").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The default cache scans /etc/cdi and /var/run/cdi; both outcomes are
    // legitimate depending on the host.
    assert!(
        stdout.contains("No CDI devices found") || stdout.contains("CDI devices found"),
        "unexpected output: {stdout}"
    );
}

#[test]
fn cdi_cli_lists_devices_verbose_json() {
    let output = Command::new(cdi_bin())
        .args(["devices", "--verbose", "--output", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn cdi_cli_inject_rejects_missing_oci_spec() {
    let output = Command::new(cdi_bin())
        .args([
            "inject",
            "/nonexistent/spec.yaml",
            "vendor.example/none=missing",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("path of oci spec not found"), "{stderr}");
}

#[test]
fn cdi_cli_inject_updates_an_oci_spec() {
    let dir = tempfile::tempdir().unwrap();
    let spec = dir.path().join("oci.yaml");
    fs::write(&spec, "ociVersion: \"1.0.2\"\n").unwrap();

    // Unknown device patterns are filtered out (not an error): the spec is
    // echoed back updated-but-unchanged.
    let output = Command::new(cdi_bin())
        .args([
            "inject",
            spec.to_str().unwrap(),
            "vendor.example/none=missing",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated OCI Spec:"), "{stdout}");
}
