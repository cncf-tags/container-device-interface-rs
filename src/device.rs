use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use oci_spec::runtime as oci;

use crate::{
    container_edits::{ContainerEdits, Validate},
    parser::{qualified_name, validate_device_name},
    spec::Spec,
    specs::config::Device as CDIDevice,
    internal::validation::validate::validate_spec_annotations,
};

// Device represents a CDI device of a Spec.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Device {
    pub cdi_device: CDIDevice,
    cdi_spec: Spec,
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

// new_device creates a new Device, associate it with the given Spec.
pub fn new_device(spec: &Spec, device: &CDIDevice) -> Result<Device> {
    let device = Device {
        cdi_device: device.clone(),
        cdi_spec: spec.clone(),
    };

    if let Err(e) = device.validate() {
        return Err(anyhow!(
            "device validated failed with error: {:?}",
            e.to_string()
        ));
    }

    Ok(device)
}

impl Device {
    // new returns a default Device
    pub fn new() -> Self {
        Self {
            cdi_device: Default::default(),
            cdi_spec: Default::default(),
        }
    }

    // get_spec returns the Spec this device is defined in.
    pub fn get_spec(&self) -> Spec {
        self.cdi_spec.clone()
    }

    // get_qualified_name returns the qualified name for this device.
    pub fn get_qualified_name(&self) -> String {
        qualified_name(
            &self.cdi_spec.get_vendor(),
            &self.cdi_spec.get_class(),
            &self.cdi_device.name,
        )
    }

    // edits returns the applicable global container edits for this spec.
    pub fn edits(&self) -> ContainerEdits {
        ContainerEdits {
            container_edits: self.cdi_device.container_edits.clone(),
        }
    }
    // apply_edits applies the device-speific container edits to an OCI Spec.
    pub fn apply_edits(&mut self, oci_spec: &mut oci::Spec) -> Result<()> {
        let _ = self.edits().apply(oci_spec);

        Ok(())
    }

    // validate the device.
    pub fn validate(&self) -> Result<()> {
        validate_device_name(&self.cdi_device.name).context("validate device name failed")?;
        let name = self.get_qualified_name();

        let annotations: &BTreeMap<String, String> =
            &<BTreeMap<String, String> as Clone>::clone(&self.cdi_device.annotations)
                .into_iter()
                .collect();
        if let Err(e) = validate_spec_annotations(&name, annotations) {
            return Err(anyhow!(
                "validate spec annotations failed with error: {:?}",
                e
            ));
        }

        let edits = self.edits();
        edits
            .validate()
            .context(format!("invalid device {:?} ", self.cdi_device.name))?;

        Ok(())
    }
}
