
use anyhow::{anyhow, Result};
use clap::{App, Arg};
use std::io::{self, Read};


extern crate cdi;

use cdi::schema as schema;

fn main() -> Result<()> {
    let matches = App::new("validate")
        .version("1.0")
        .about(
            "Validates JSON documents against a schema

  1.specify document file as an argument
    validate --schema <schema.json> <document.json>
      
  2.pass document content through a pipe
    cat <document.json> | validate --schema <schema.json>

  3.input document content manually, ended with ctrl+d(or your self-defined EOF keys)
    validate --schema <schema.json>
    [INPUT DOCUMENT CONTENT HERE]	    
	    ",
        )
        .arg(
            Arg::with_name("schema")
                .long("schema")
                .value_name("FILE")
                .default_value("builtin")
                .help("JSON Schema to validate against"),
        )
        .arg(
            Arg::new("document")
                .help("The document to validate")
                .index(1)
                .default_value("-")
                .required(false),
        )
        .get_matches();

    let schema_file = matches.value_of("schema").unwrap_or("builtin");
    println!("Validating against JSON schema {}...", schema_file);

    let document = matches.value_of("document").unwrap();

    let doc_data = if document == "-" {
        println!("Reading from <stdin>...");
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        buffer
    } else {
        std::fs::read(&document)?
    };

    //schema::validate(schema_file, &doc_data)

}
