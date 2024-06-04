use anyhow::{Context, Result};
use cdi::default_cache::get_default_cache;
use cdi::device::Device;

use crate::cdi_ops::{api::cdi_inject_devices, utils::read_oci_spec};

use super::args::{DevicesArgs, InjectArgs};
use super::format::{choose_format, indent, marshal_object};

pub fn handle_cdi_inject(args: &InjectArgs) -> Result<()> {
    let oci_spec = &mut read_oci_spec(&args.oci_spec)?;
    cdi_inject_devices(oci_spec, args.cdi_devices.clone(), &args.format)
        .context("cdi inject devices failed")?;

    Ok(())
}

pub fn handle_cdi_devices(args: &DevicesArgs) -> Result<()> {
    cdi_list_devices(args.verbose, &args.format)
        .context("cdi list devices failed")?;
    Ok(())
}

fn cdi_list_devices(verbose: bool, format: &str) -> Result<()> {
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
