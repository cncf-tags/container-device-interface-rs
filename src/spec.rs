use std::{collections::BTreeMap, fs::File, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use oci_spec::runtime as oci;
use path_clean::clean;

use crate::{
    container_edits::ContainerEdits, device::Device, parser::parse_qualifier,
    specs::config::Spec as CDISpec, utils::is_cdi_spec,
};

const DEFAULT_SPEC_EXT_SUFFIX: &str = ".yaml";

// Spec represents a single CDI Spec. It is usually loaded from a
// file and stored in a cache. The Spec has an associated priority.
// This priority is inherited from the associated priority of the
// CDI Spec directory that contains the CDI Spec file and is used
// to resolve conflicts if multiple CDI Spec files contain entries
// for the same fully qualified device.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Spec {
    cdi_spec: CDISpec,
    vendor: String,
    class: String,
    path: String,
    priority: i32,
    pub devices: BTreeMap<String, Device>,
}

impl Spec {
    // get_vendor returns the vendor of this Spec.
    pub fn get_vendor(&self) -> String {
        self.vendor.clone()
    }

    // get_class returns the device class of this Spec.
    pub fn get_class(&self) -> String {
        self.class.clone()
    }

    // get_devices returns the devices list.
    pub fn get_devices(&self) -> BTreeMap<String, Device> {
        self.devices.clone()
    }

    // get_device returns the device for the given unqualified name.
    pub fn get_device(&self, key: &str) -> Option<&Device> {
        self.devices.get(key)
    }

    // get_path returns the filesystem path of this Spec.
    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    // get_priority returns the priority of this Spec.
    pub fn get_priority(&self) -> i32 {
        self.priority
    }

    // edits returns the applicable global container edits for this spec.
    pub fn edits(&mut self) -> Option<ContainerEdits> {
        self.cdi_spec
            .container_edits
            .clone()
            .map(|ce| ContainerEdits {
                container_edits: ce,
            })
    }

    // validate the Spec.
    pub fn validate(&self) -> Result<BTreeMap<String, Device>> {
        // TODO
        Ok(BTreeMap::new())
    }

    // apply_edits applies the Spec's global-scope container edits to an OCI Spec.
    pub fn apply_edits(&mut self, oci_spec: &mut oci::Spec) -> Result<()> {
        if let Some(ref mut ce) = self.edits() {
            ce.apply(oci_spec)
                .context("container edits applys failed.")?;
        }

        Ok(())
    }
}

pub fn parse_spec(path: &PathBuf) -> Result<CDISpec> {
    if !path.exists() {
        return Err(anyhow!("CDI spec path not found"));
    }

    let config_file = File::open(path).context("open config file")?;
    let cdi_spec: CDISpec =
        serde_yaml::from_reader(config_file).context("serde yaml read from file")?;

    Ok(cdi_spec)
}

// validate_spec validates the Spec using the extneral validator.
pub fn validate_spec(_raw_spec: &CDISpec) -> Result<()> {
    // TODO
    Ok(())
}

// read_spec reads the given CDI Spec file. The resulting Spec is
// assigned the given priority. If reading or parsing the Spec
// data fails read_spec returns a nil Spec and an error.
pub fn read_spec(path: &PathBuf, priority: i32) -> Result<Spec> {
    let raw_spec = parse_spec(path).context("parse spec file failed")?;
    let cdi_spec = new_spec(&raw_spec, path, priority).context("create a new cdi spec failed")?;

    Ok(cdi_spec)
}

// new_spec creates a new Spec from the given CDI Spec data. The
// Spec is marked as loaded from the given path with the given
// priority. If Spec data validation fails new_spec returns an error.
pub fn new_spec(raw_spec: &CDISpec, path: &PathBuf, priority: i32) -> Result<Spec> {
    validate_spec(raw_spec).context("invalid CDI Spec")?;

    let mut cleaned_path = clean(path);
    if !is_cdi_spec(&cleaned_path) {
        cleaned_path.set_extension(DEFAULT_SPEC_EXT_SUFFIX);
    }

    let (vendor, class) = parse_qualifier(&raw_spec.kind);

    let mut spec: Spec = Spec {
        cdi_spec: raw_spec.clone(),
        path: cleaned_path.display().to_string(),
        priority,
        vendor: vendor.to_owned(),
        class: class.to_owned(),
        ..Default::default()
    };
    spec.devices = spec.validate().context("validate spec failed")?;

    Ok(spec)
}
