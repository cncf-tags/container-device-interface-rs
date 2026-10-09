use std::io::{self, Read};

use anyhow::Result;

use crate::ValidateArgs;

/// handle_validate is used to handle the input arguments
pub fn handle_validate(args: ValidateArgs) -> Result<()> {
    let doc_data = if args.document == "-" {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        buffer
    } else {
        std::fs::read(&args.document)?
    };

    // "builtin", "none" to skip validation, or a schema file (schema::load).
    match container_device_interface::schema::load(&args.schema)? {
        Some(schema) => schema.validate_document(&doc_data),
        None => Ok(()),
    }
}
