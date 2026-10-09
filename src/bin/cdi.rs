extern crate container_device_interface as cdi;
mod cdi_ops;

use anyhow::Result;
use clap::Parser;

use cdi_ops::{
    args::{CdiCli, Commands},
    handler::{handle_cdi_devices, handle_cdi_inject},
};

fn main() -> Result<()> {
    let cli = CdiCli::parse();

    // Like the Go cdi tool, Spec files are schema-validated as the registry
    // loads them unless --schema none is given.
    if let Some(validator) = cdi::schema::load(&cli.schema)? {
        cdi::spec::set_spec_validator(validator);
    }

    match &cli.command {
        Commands::Devices(args) => {
            handle_cdi_devices(args)?;
        }
        Commands::Inject(args) => {
            handle_cdi_inject(args)?;
        } // TODO: to support more command here
    }

    Ok(())
}
