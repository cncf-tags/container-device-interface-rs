use core::panic;
use anyhow::Ok;
use serde_json::Value;
use serde_json::json;
use jsonschema::JSONSchema;
use jsonschema::Draft;

use anyhow::{anyhow, Result};

const SCHEMA_JSON: &'static str = include_str!("schema.json");
const DEFS_JSON: &'static str = include_str!("defs.json");



pub fn validate(schema: &jsonschema::JSONSchema, doc_data: &[u8]) -> Result<()> {
	let mut schema_json: serde_json::Value = serde_json::from_str(include_str!("schema.json"))?;
	let defs_json: serde_json::Value = serde_json::from_str(include_str!("defs.json"))?;

	// Merge the definitions into the main schema under the "definitions" key
	if let Some(obj) = schema_json.as_object_mut() {
		obj.insert("definitions".to_string(), defs_json);
	}
/*
	let compiled_schema = JSONSchema::options()
        .with_draft(Draft::Draft7) // Adjust the draft version as needed
        .compile(&schema_json)?;

	let doc = &serde_json::from_slice(doc_data)?;

	let result = compiled_schema.validate(doc);

*/

	Ok(())
}

/* 
fn validate_data(schema: &Value, data: &Value) -> Result<(), Vec<jsonschema::ValidationError>> {
	let compiled_schema = JSONSchema::options()
	    .with_draft(Draft::Draft7) // Adjust the draft version as needed
	    .compile(schema)?;
	
	compiled_schema.validate(data).map_err(|e| e.collect())
}



pub fn load(schema_file: &str) -> Result<jsonschema::JSONSchema> {

	let schema_context = SchemaContext::builtin()?;
	Ok(schema_context.compiled_schema)
	/*
	if schema_file == "builtin" {
		println!("Loading schema from {}...", schema_file);

		print!("schema:\n{}", builtin_schema);

		match jsonschema::JSONSchema::compile(&serde_json::from_str(&builtin_schema)?) {
			Ok(schema) => return Ok(schema),
			Err(e) => return Err(anyhow!("failed to compile builtin schema {}", e)),
		}
	}
	*/
	//panic!("not implemented yet loading from other sources")
}
    

 pub fn validate(schema: &jsonschema::JSONSchema, doc_data: &[u8]) -> Result<()> {
	let doc = serde_json::from_slice(doc_data)?;
	match schema.validate(&doc) {
	    Ok(_) => (),
	    Err(_e) => return Err(anyhow!("validation failed")),
	}
	Ok(())
    }
    */