use anyhow::Result;
use cdi::default_cache::get_default_cache;
use cdi::default_cache::{inject_devices, list_devices};
use cdi::device::Device;
use oci_spec::runtime as oci;

use crate::cdi_ops::{
    format::choose_format, format::indent, format::marshal_object, utils::find_target_devices,
};

pub fn cdi_inject_devices(
    oci_spec: &mut oci::Spec,
    patterns: Vec<String>,
    format: &str,
) -> Result<()> {
    let devices = find_target_devices(list_devices(), patterns);
    if let Err(unresolved) = inject_devices(oci_spec, devices) {
        println!("{:?}", unresolved.to_string());
    }

    println!("Updated OCI Spec:");
    print!("{}", marshal_object(2, oci_spec, format));

    Ok(())
}

pub fn cdi_list_devices(verbose: bool, format: &str) -> Result<()> {
    let cache = get_default_cache();
    let devices = cache.lock().unwrap().list_devices();

    if devices.is_empty() {
        println!("No CDI devices found");
        return Ok(());
    }

    println!("CDI devices found:");
    for (idx, device) in devices.iter().enumerate() {
        cdi_print_device(
            idx,
            cache.lock().unwrap().get_device(device).unwrap().clone(),
            verbose,
            format,
            2,
        );
    }
    Ok(())
}

fn cdi_print_device(idx: usize, dev: Device, verbose: bool, format: &str, level: usize) {
    if !verbose {
        println!("{}{}. {}", indent(level), idx, dev.get_qualified_name());
        return;
    }

    let spec = dev.get_spec();
    let format = choose_format(format, &spec.get_path());

    println!("  {} ({})", dev.get_qualified_name(), spec.get_path());
    print!("{}", marshal_object(level + 2, &dev.cdi_device, &format));

    let edits: &Option<cdi::specs::config::ContainerEdits> = &spec.cdi_spec.container_edits;

    if let Some(edits) = edits {
        let total_len = edits.env.as_ref().map_or(0, |v| v.len())
            + edits.device_nodes.as_ref().map_or(0, |v| v.len())
            + edits.hooks.as_ref().map_or(0, |v| v.len())
            + edits.mounts.as_ref().map_or(0, |v| v.len());
        if total_len > 0 {
            println!("{}global Spec containerEdits:", indent(level + 2));
            print!("{}", marshal_object(level + 4, &edits, &format));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdi::cache::with_auto_refresh;
    use cdi::default_cache::{configure, refresh};
    use cdi::spec_dirs::with_spec_dirs;
    use std::fs;

    // One test: the default cache is a process-wide singleton shared by
    // every test in this binary.
    #[test]
    fn list_and_inject_through_the_default_cache() {
        // Empty spec dir first: the "No CDI devices found" early return.
        let empty = tempfile::tempdir().unwrap();
        configure(vec![
            with_spec_dirs(&[empty.path().to_str().unwrap()]),
            with_auto_refresh(false),
        ])
        .unwrap();
        refresh().unwrap();
        cdi_list_devices(false, " ").unwrap();

        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("vendor.yaml"),
            r#"cdiVersion: "0.6.0"
kind: "vendor.com/device"
containerEdits:
  env:
    - "GLOBAL=1"
devices:
  - name: "gpu0"
    containerEdits:
      env:
        - "VENDOR=1"
"#,
        )
        .unwrap();
        configure(vec![
            with_spec_dirs(&[dir.path().to_str().unwrap()]),
            with_auto_refresh(false),
        ])
        .unwrap();
        refresh().unwrap();

        // Non-verbose and verbose (the latter prints the global spec edits).
        cdi_list_devices(false, " ").unwrap();
        cdi_list_devices(true, "json").unwrap();
        cdi_list_devices(true, "").unwrap();

        let mut oci_spec = oci::Spec::default();
        cdi_inject_devices(
            &mut oci_spec,
            vec![
                "vendor.com/device=gpu0".to_string(),
                "vendor.com/device=unknown".to_string(), // filtered, not an error
            ],
            "yaml",
        )
        .unwrap();
        let env = oci_spec.process().as_ref().unwrap().env().as_ref().unwrap();
        assert!(env.contains(&"VENDOR=1".to_string()));
        assert!(env.contains(&"GLOBAL=1".to_string()));

        // A loadable spec whose device node cannot be stat'ed fails at
        // inject time (fill_missing_info) - the error-print path.
        fs::write(
            dir.path().join("broken.yaml"),
            r#"cdiVersion: "0.6.0"
kind: "vendor2.com/device"
devices:
  - name: "bad0"
    containerEdits:
      deviceNodes:
        - path: "/nonexistent/device/node"
"#,
        )
        .unwrap();
        refresh().unwrap();
        let mut oci_spec = oci::Spec::default();
        cdi_inject_devices(
            &mut oci_spec,
            vec!["vendor2.com/device=bad0".to_string()],
            "yaml",
        )
        .unwrap();
    }
}
