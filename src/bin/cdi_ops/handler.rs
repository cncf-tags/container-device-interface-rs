use anyhow::Result;

use super::args::{DevicesArgs, InjectArgs};

pub fn handle_cdi_inject(args: &InjectArgs) -> Result<()> {
    println!("{:?}", args.oci_spec);
    println!("{:?}", args.cdi_devices);

    // TODO: add complete work later.
    // let oci_spec = &mut read_oci_spec(&args.oci_spec)?;
    // cdi_inject_devices(oci_spec, args.cdi_devices.clone()).context("cdi inject devices failed")?;

    Ok(())
}

pub fn handle_cdi_devices(args: &DevicesArgs) -> Result<()> {
    println!("{:#?}", args);

    Ok(())
}
