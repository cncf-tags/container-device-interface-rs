use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use oci_spec::runtime as oci;

use crate::{
    container_edits::{ContainerEdits, Validate},
    internal::validation::validate::validate_spec_annotations,
    parser::{qualified_name, validate_device_name},
    spec::Spec,
    specs::config::Device as CDIDevice,
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
        self.edits().apply(oci_spec)?;
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
        if edits.is_empty() {
            return Err(anyhow!("invalid device, empty device edits"));
        }
        edits
            .validate()
            .context(format!("invalid device {:?} ", self.cdi_device.name))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        spec::new_spec,
        specs::config::{
            ContainerEdits as CDIContainerEdits, Device as CDIDeviceSpec, Spec as CDISpec,
        },
    };
    use std::path::PathBuf;

    fn spec_with(device: CDIDeviceSpec) -> Spec {
        let raw = CDISpec {
            version: "0.6.0".to_string(),
            kind: "vendor.com/class".to_string(),
            devices: vec![device.clone()],
            ..Default::default()
        };
        new_spec(&raw, &PathBuf::from("/tmp/spec.yaml"), 0).unwrap()
    }

    fn cdi_device(name: &str) -> CDIDeviceSpec {
        CDIDeviceSpec {
            name: name.to_string(),
            container_edits: CDIContainerEdits {
                env: Some(vec!["X=1".to_string()]),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn new_device_builds_and_applies_edits() {
        let raw = cdi_device("gpu0");
        let spec = spec_with(raw.clone());
        let mut device = new_device(&spec, &raw).unwrap();

        assert_eq!(device.get_qualified_name(), "vendor.com/class=gpu0");
        assert_eq!(device.get_spec().get_vendor(), "vendor.com");

        let mut oci_spec = oci::Spec::default();
        device.apply_edits(&mut oci_spec).unwrap();
        let env = oci_spec.process().as_ref().unwrap().env().as_ref().unwrap();
        assert!(env.contains(&"X=1".to_string()));
    }

    #[test]
    fn new_device_rejects_invalid_names_and_empty_edits() {
        let bad_name = cdi_device("not a name");
        let spec = spec_with(cdi_device("gpu0"));
        let err = new_device(&spec, &bad_name).unwrap_err();
        assert!(err.to_string().contains("validate device name failed"));

        let empty_edits = CDIDeviceSpec {
            name: "gpu1".to_string(),
            ..Default::default()
        };
        let err = new_device(&spec, &empty_edits).unwrap_err();
        assert!(err.to_string().contains("empty device edits"));
    }

    #[test]
    fn default_device_is_constructible() {
        let device = Device::default();
        assert_eq!(device.cdi_device.name, "");
    }
}
