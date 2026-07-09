use std::{
    io::{self, Read},
    path::Path,
};

use anyhow::{anyhow, Result};
use serde_json::Value;

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

    match args.schema.as_str() {
        "builtin" => container_device_interface::schema::validate_builtin(&doc_data),
        "none" | "" => Ok(()),
        _ => {
            let schema_path = Path::new(&args.schema);
            let schema_data = std::fs::read(schema_path)?;
            let defs_path = schema_path.with_file_name("defs.json");

            if defs_path.exists() {
                let defs_data = std::fs::read(defs_path)?;
                let schema = container_device_interface::schema::compile_cdi_schema(
                    &schema_data,
                    &defs_data,
                )?;
                container_device_interface::schema::validate_cdi(&schema, &doc_data)
            } else {
                let schema_json: Value = serde_json::from_slice(&schema_data)?;
                if refs_defs_json(&schema_json) {
                    return Err(anyhow!(
                        "schema {} references defs.json, but no sibling defs.json was found",
                        schema_path.display()
                    ));
                }
                let schema = jsonschema::Validator::options()
                    .with_draft(jsonschema::Draft::Draft7)
                    .build(&schema_json)?;
                container_device_interface::schema::validate(&schema, &doc_data)
            }
        }
    }
}

fn refs_defs_json(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get("$ref")
                .and_then(Value::as_str)
                .is_some_and(|reference| {
                    reference == "defs.json" || reference.starts_with("defs.json#")
                })
                || object.values().any(refs_defs_json)
        }
        Value::Array(values) => values.iter().any(refs_defs_json),
        _ => false,
    }
}
