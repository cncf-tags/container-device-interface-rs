use anyhow::Result;
use cdi::default_cache::{inject_devices, list_devices};
use oci_spec::runtime as oci;
use cdi::default_cache::get_default_cache;
use cdi::device::Device;

use crate::cdi_ops::{format::marshal_object, format::indent, format::choose_format, utils::find_target_devices};

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
    println!("{:?}", marshal_object(2, oci_spec, format));

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
