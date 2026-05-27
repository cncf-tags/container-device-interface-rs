use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use oci_spec::runtime as oci;
use path_clean::clean;

use crate::{
    container_edits::ContainerEdits,
    container_edits::Validate,
    device::new_device,
    device::Device,
    internal::validation::validate::validate_spec_annotations,
    parser::parse_qualifier,
    parser::validate_class_name,
    parser::validate_vendor_name,
    specs::config::Spec as CDISpec,
    utils::is_cdi_spec,
    version::{
        minimum_required_version, validate_declared_version_fields, VersionWrapper,
        VALID_SPEC_VERSIONS,
    },
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
    pub cdi_spec: CDISpec,
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
    pub fn validate(&mut self) -> Result<BTreeMap<String, Device>> {
        validate_version(&self.cdi_spec).context("validate cdi version failed")?;
        validate_vendor_name(&self.vendor).context("validate vendor name failed")?;
        validate_class_name(&self.class).context("validate class name failed")?;
        validate_spec_annotations(&self.cdi_spec.kind, &self.cdi_spec.annotations)
            .context("validate spec annotations failed")?;

        if let Some(ref mut ce) = self.edits() {
            ce.validate().context("validate container edits failed")?;
        }

        let mut devices = BTreeMap::new();
        for d in &self.cdi_spec.devices {
            let dev =
                new_device(self, d).with_context(|| format!("failed to add device {}", d.name))?;
            if devices.contains_key(&d.name) {
                return Err(anyhow::anyhow!("invalid spec, multiple device {}", d.name));
            }
            devices.insert(d.name.clone(), dev);
        }

        if devices.is_empty() {
            return Err(anyhow::anyhow!("invalid spec, no devices"));
        }

        Ok(devices)
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

    let data = std::fs::read(path).context("read config file")?;
    let cdi_spec: CDISpec = serde_yaml::from_slice(&data).context("serde yaml read from file")?;

    Ok(cdi_spec)
}

// validate_spec validates the Spec using the extneral validator.
pub fn validate_spec(raw_spec: &CDISpec) -> Result<()> {
    let data = serde_yaml::to_string(raw_spec).context("marshal CDI spec for schema validation")?;
    crate::schema::validate_builtin(data.as_bytes()).context("invalid CDI Spec schema")?;
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
    if raw_spec.devices.is_empty() {
        return Err(anyhow::anyhow!("invalid spec, no devices"));
    }

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

fn validate_version(cdi_spec: &CDISpec) -> Result<()> {
    let version = &cdi_spec.version;
    if !VALID_SPEC_VERSIONS.is_valid_version(version) {
        return Err(anyhow::anyhow!("invalid version {}", version));
    }

    validate_declared_version_fields(cdi_spec)?;

    let min_version = minimum_required_version(cdi_spec)
        .with_context(|| "could not determine minimum required version")?;

    if min_version.is_greater_than(&VersionWrapper::new(version)) {
        return Err(anyhow::anyhow!(
            "the spec version must be at least v{}",
            min_version
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oci_spec::runtime as oci;
    use std::path::PathBuf;

    #[test]
    fn parse_spec_rejects_unknown_fields() {
        let path = PathBuf::from("tests/fixtures/cdi-unknown-field.yaml");
        let err = parse_spec(&path).expect_err("unknown field should fail");
        assert!(err.to_string().contains("serde yaml read from file"));
    }

    #[test]
    fn new_spec_rejects_empty_devices() {
        let path = PathBuf::from("tests/fixtures/cdi-empty-devices.yaml");
        let raw = parse_spec(&path).expect("empty device fixture parses");
        let err = new_spec(&raw, &path, 0).expect_err("empty devices should fail");
        assert!(err.to_string().contains("no devices"));
    }

    #[test]
    fn new_spec_rejects_legacy_intel_rdt_fields_in_v1_1() {
        let path = PathBuf::from("tests/fixtures/cdi-v1.1-legacy-intel-rdt.yaml");
        let raw = parse_spec(&path).expect("legacy fields should parse before version validation");
        let err = new_spec(&raw, &path, 0).expect_err("v1.1.0 should reject legacy fields");

        assert!(format!("{err:#}").contains("enableCMT"));
    }

    #[test]
    fn new_spec_processes_legacy_intel_rdt_fields_before_v1_1() {
        let path = PathBuf::from("tests/fixtures/cdi-v1.0-legacy-intel-rdt.yaml");
        let raw = parse_spec(&path).expect("v1.0.0 legacy Intel RDT fixture parses");
        let mut spec = new_spec(&raw, &path, 0).expect("v1.0.0 legacy Intel RDT fixture validates");
        let mut oci_spec = oci::Spec::default();

        spec.apply_edits(&mut oci_spec)
            .expect("legacy Intel RDT edits apply");

        let rdt = oci_spec
            .linux()
            .as_ref()
            .and_then(|linux| linux.intel_rdt().as_ref())
            .expect("Intel RDT should be set");

        #[allow(deprecated)]
        {
            assert_eq!(&Some(true), rdt.enable_cmt());
            assert_eq!(&Some(true), rdt.enable_mbm());
        }
    }

    #[test]
    fn new_spec_rejects_empty_device_edits() {
        let raw = CDISpec {
            version: "1.1.0".to_string(),
            kind: "vendor.com/device".to_string(),
            devices: vec![crate::specs::config::Device {
                name: "gpu0".to_string(),
                container_edits: crate::specs::config::ContainerEdits::default(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let path = PathBuf::from("/tmp/vendor-device.yaml");

        let err = new_spec(&raw, &path, 0).expect_err("empty device edits should fail");
        let err = format!("{err:?}");

        assert!(err.contains("empty device edits"), "{err}");
    }
}
