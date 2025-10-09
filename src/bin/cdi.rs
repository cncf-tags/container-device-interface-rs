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
