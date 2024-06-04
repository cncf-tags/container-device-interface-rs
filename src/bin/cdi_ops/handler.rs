use anyhow::{Context, Result};

use crate::cdi_ops::{api::cdi_inject_devices, api::cdi_list_devices, utils::read_oci_spec};

use super::args::{DevicesArgs, InjectArgs};


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
