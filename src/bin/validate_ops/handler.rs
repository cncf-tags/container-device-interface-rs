use std::io::{self, Read};

use anyhow::Result;

use crate::ValidateArgs;

/// handle_validate is used to handle the input arguments
pub fn handle_validate(args: ValidateArgs) -> Result<()> {
    println!("args: {:?}", args);
    let _doc_data = if args.document == "-" {
        println!("Reading from <stdin>...");
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        buffer
    } else {
        std::fs::read(&args.document)?
    };

    // TODO:
    // schema::validate(args.schema, &doc_data)
    Ok(())
}
