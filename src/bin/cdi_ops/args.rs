use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = "The 'cdi' utility allows you to inspect and interact with the
CDI Registry. Various commands are available for listing CDI
Spec files, vendors, classes, devices, validating the content
of the registry, injecting devices into OCI Specs, and for
monitoring changes in the Registry.

See cdi --help for a list of available commands. You can get
additional help about <command> by using 'cdi <command> -h'.
"
)]
#[command(propagate_version = true)]
pub struct CdiCli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List devices in the CDI registry
    #[clap(
        about = "List devices in the CDI registry.",
        long_about = "The 'devices' command lists devices found in the CDI registry."
    )]
    Devices(DevicesArgs),

    /// Inject CDI devices into an OCI Spec.
    #[clap(
        about = "Inject CDI devices into an OCI Spec.",
        long_about = "The 'inject' command reads an OCI Spec from a file (use \"-\" for stdin),
injects a requested set of CDI devices into it and dumps the resulting
updated OCI Spec."
    )]
    Inject(InjectArgs),
}

#[derive(Debug, Args)]
#[command(
    version,
    about,
    long_about = "The 'devices' command lists devices found in the CDI registry."
)]
pub struct DevicesArgs {
    #[arg(
        short = 'o',
        long = "output",
        default_value = " ",
        help = "output format for OCI Spec (json|yaml)"
    )]
    pub format: String,
    #[arg(short = 'v', long = "verbose", help = "list CDI Spec details")]
    pub verbose: bool,
}

#[derive(Debug, Args)]
pub struct InjectArgs {
    /// OCI Spec File
    #[arg(required = true, value_parser)]
    pub oci_spec: String,

    /// CDI Device List
    #[arg(required = true, value_parser)]
    pub cdi_devices: Vec<String>,

    #[arg(
        short = 'o',
        long = "output",
        default_value = " ",
        help = "output format for OCI Spec (json|yaml)"
    )]
    pub format: String,
}
