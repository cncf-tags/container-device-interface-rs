use clap::Parser;

#[derive(Clone, Debug, Parser)]
#[command(
    name = "validate",
    version = "0.1.0",
    about = "validate cli",
    long_about = "Validate is used to check document with specified schema.
You can use validate in following ways:

1.specify document file as an argument
  validate --schema <schema.json> <document.json>
    
2.pass document content through a pipe
  cat <document.json> | validate --schema <schema.json>

3.input document content manually, ended with ctrl+d(or your self-defined EOF keys)
  validate --schema <schema.json>
  [INPUT DOCUMENT CONTENT HERE]	    
"
)]
pub struct ValidateArgs {
    /// JSON Schema to validate against (default "builtin")
    #[arg(long = "schema", default_value = "builtin")]
    pub schema: String,
    /// Document to be validated (default "-") and it's regarded as index argument
    #[arg(value_name = "document", default_value = "-", index = 1)]
    pub document: String,
}
