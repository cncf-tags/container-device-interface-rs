use anyhow::Result;
use cdi::default_cache::{inject_devices, list_devices};
use oci_spec::runtime as oci;

use crate::cdi_ops::{format::marshal_object, utils::find_target_devices};

pub fn cdi_inject_devices(
    oci_spec: &mut oci::Spec,
    patterns: Vec<String>,
    format: &str,
) -> Result<()> {
    let devices = find_target_devices(list_devices(), patterns);
    if let Err(unresolved) = inject_devices(oci_spec, devices) {
        println!("{:?}", unresolved.to_string());
    }

    println!("Updated OCI Spec:");
    print!("{}", marshal_object(2, oci_spec, format));

    Ok(())
}
