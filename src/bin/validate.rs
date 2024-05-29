use anyhow::{Context, Result};
use clap::Parser;

mod validate_ops;

use validate_ops::{args::ValidateArgs, handler::handle_validate};

fn main() -> Result<()> {
    let args = ValidateArgs::parse();
    handle_validate(args).context("handle validate failed")?;

    Ok(())
}
