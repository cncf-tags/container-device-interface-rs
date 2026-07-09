use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

use crate::parser::parse_qualifier;
use crate::specs::config;
use crate::specs::config::Spec as CDISpec;
use semver::Version;

const CURRENT_VERSION: &str = config::CURRENT_VERSION;
static VCURRENT: Lazy<String> = Lazy::new(|| format!("v{}", CURRENT_VERSION));

// Released versions of the CDI specification
const V010: &str = "v0.1.0";
const V020: &str = "v0.2.0";
const V030: &str = "v0.3.0";
const V040: &str = "v0.4.0";
const V050: &str = "v0.5.0";
const V060: &str = "v0.6.0";
const V070: &str = "v0.7.0";
const V080: &str = "v0.8.0";
const V100: &str = "v1.0.0";
const V110: &str = "v1.1.0";

// Earliest supported version of the CDI specification
const VEARLIEST: &str = V030;

type RequiredFunc = fn(&CDISpec) -> bool;

#[derive(Default)]
pub struct VersionMap(BTreeMap<String, Option<RequiredFunc>>);

pub static VALID_SPEC_VERSIONS: Lazy<VersionMap> = Lazy::new(|| {
    let mut map = BTreeMap::new();
    map.insert(V010.to_string(), None);
    map.insert(V020.to_string(), None);
    map.insert(V030.to_string(), None);
    map.insert(V040.to_string(), Some(requires_v040 as RequiredFunc));
    map.insert(V050.to_string(), Some(requires_v050));
    map.insert(V060.to_string(), Some(requires_v060));
    map.insert(V070.to_string(), Some(requires_v070));
    map.insert(V080.to_string(), None);
    map.insert(V100.to_string(), None);
    map.insert(V110.to_string(), Some(requires_v110 as RequiredFunc));
    VersionMap(map)
});

impl VersionMap {
    pub fn is_valid_version(&self, spec_version: &str) -> bool {
        self.0
            .contains_key(&format!("v{}", VersionWrapper::new(spec_version)))
    }

    pub fn required_version(&self, spec: &CDISpec) -> VersionWrapper {
        let mut min_version = VersionWrapper::new(VEARLIEST);
        for (v, is_required) in &self.0 {
            if let Some(is_required_fn) = is_required {
                let version_wrapper = VersionWrapper::new(v);
                if is_required_fn(spec) && version_wrapper.is_greater_than(&min_version) {
                    min_version = version_wrapper;
                }
                if min_version.is_latest() {
                    break;
                }
            }
        }
        min_version
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionWrapper(String);

impl std::fmt::Display for VersionWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.trim_start_matches('v'))
    }
}

impl VersionWrapper {
    pub fn new(v: &str) -> Self {
        VersionWrapper(format!("v{}", v.trim_start_matches('v')))
    }

    pub fn is_greater_than(&self, other: &VersionWrapper) -> bool {
        Version::parse(&self.to_string()).unwrap() > Version::parse(&other.to_string()).unwrap()
    }

    fn is_latest(&self) -> bool {
        self.0 == *VCURRENT
    }
}

pub fn minimum_required_version(spec: &CDISpec) -> Result<VersionWrapper> {
    Ok(VALID_SPEC_VERSIONS.required_version(spec))
}

pub(crate) fn validate_declared_version_fields(spec: &CDISpec) -> Result<()> {
    if !VALID_SPEC_VERSIONS.is_valid_version(&spec.version) {
        return Err(anyhow::anyhow!("invalid version {}", spec.version));
    }

    let declared = VersionWrapper::new(&spec.version);
    let v110 = VersionWrapper::new(V110);
    let declared_is_v110_or_newer = !v110.is_greater_than(&declared);

    for (scope, edits) in spec
        .container_edits
        .iter()
        .map(|edits| ("containerEdits", edits))
        .chain(
            spec.devices
                .iter()
                .map(|device| ("devices[].containerEdits", &device.container_edits)),
        )
    {
        if let Some(intel_rdt) = &edits.intel_rdt {
            if declared_is_v110_or_newer {
                if intel_rdt.enable_cmt.is_some() {
                    return Err(anyhow::anyhow!(
                        "{}.intelRdt.enableCMT is not valid for CDI spec version {}",
                        scope,
                        spec.version
                    ));
                }
                if intel_rdt.enable_mbm.is_some() {
                    return Err(anyhow::anyhow!(
                        "{}.intelRdt.enableMBM is not valid for CDI spec version {}",
                        scope,
                        spec.version
                    ));
                }
            } else {
                if intel_rdt.schemata.is_some() {
                    return Err(anyhow::anyhow!(
                        "{}.intelRdt.schemata requires CDI spec version 1.1.0",
                        scope
                    ));
                }
                if intel_rdt.enable_monitoring.is_some() {
                    return Err(anyhow::anyhow!(
                        "{}.intelRdt.enableMonitoring requires CDI spec version 1.1.0",
                        scope
                    ));
                }
            }
        }

        if !declared_is_v110_or_newer
            && edits
                .net_devices
                .as_ref()
                .is_some_and(|devices| !devices.is_empty())
        {
            return Err(anyhow::anyhow!(
                "{}.netDevices requires CDI spec version 1.1.0",
                scope
            ));
        }
    }

    Ok(())
}

fn requires_v110(spec: &CDISpec) -> bool {
    if let Some(edits) = &spec.container_edits {
        if edits
            .net_devices
            .as_ref()
            .is_some_and(|devices| !devices.is_empty())
        {
            return true;
        }
        if let Some(intel_rdt) = &edits.intel_rdt {
            if intel_rdt.schemata.is_some() || intel_rdt.enable_monitoring.is_some() {
                return true;
            }
        }
    }

    for dev in &spec.devices {
        let edits = &dev.container_edits;
        if edits
            .net_devices
            .as_ref()
            .is_some_and(|devices| !devices.is_empty())
        {
            return true;
        }
        if let Some(intel_rdt) = &edits.intel_rdt {
            if intel_rdt.schemata.is_some() || intel_rdt.enable_monitoring.is_some() {
                return true;
            }
        }
    }

    false
}

fn requires_v070(spec: &CDISpec) -> bool {
    let edits = &spec.container_edits;
    if let Some(edits) = edits {
        if edits.intel_rdt.as_ref().is_some() {
            return true;
        }
        if edits
            .additional_gids
            .as_ref()
            .is_some_and(|v| !v.is_empty())
        {
            return true;
        }
    }

    for d in &spec.devices {
        let edits = &d.container_edits;

        if edits.intel_rdt.as_ref().is_some() {
            return true;
        }
        if edits
            .additional_gids
            .as_ref()
            .is_some_and(|v| !v.is_empty())
        {
            return true;
        }
    }
    false
}

fn requires_v060(spec: &CDISpec) -> bool {
    if !spec.annotations.is_empty() {
        return true;
    }
    for d in &spec.devices {
        if !d.annotations.is_empty() {
            return true;
        }
    }
    let (vendor, class) = parse_qualifier(&spec.kind);
    if !vendor.is_empty() && class.contains('.') {
        return true;
    }
    false
}

fn requires_v050(spec: &CDISpec) -> bool {
    if spec
        .devices
        .iter()
        .any(|d| !d.name.chars().next().unwrap_or_default().is_alphabetic())
    {
        return true;
    }

    let edits = spec
        .devices
        .iter()
        .map(|d| &d.container_edits)
        .chain(spec.container_edits.as_ref());

    edits
        .flat_map(|edits| edits.device_nodes.iter().flat_map(|nodes| nodes.iter()))
        .any(|node| {
            node.host_path
                .as_deref()
                .is_some_and(|path| !path.is_empty())
        })
}

fn requires_v040(spec: &CDISpec) -> bool {
    spec.devices
        .iter()
        .map(|d| &d.container_edits)
        .chain(spec.container_edits.as_ref())
        .flat_map(|edits| edits.mounts.iter().flat_map(|mounts| mounts.iter()))
        .any(|mount| mount.r#type.as_ref().is_some_and(|typ| !typ.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specs::config::{ContainerEdits, Device, IntelRdt, LinuxNetDevice, Spec};

    fn spec_with_edits(version: &str, edits: ContainerEdits) -> Spec {
        Spec {
            version: version.to_string(),
            kind: "vendor.com/device".to_string(),
            devices: vec![Device {
                name: "gpu0".to_string(),
                container_edits: ContainerEdits::default(),
                ..Default::default()
            }],
            container_edits: Some(edits),
            ..Default::default()
        }
    }

    #[test]
    fn accepts_current_v1_1_0_version() {
        assert!(VALID_SPEC_VERSIONS.is_valid_version("1.1.0"));
    }

    #[test]
    fn recognizes_v0_8_and_v1_0_versions() {
        assert!(VALID_SPEC_VERSIONS.is_valid_version("0.8.0"));
        assert!(VALID_SPEC_VERSIONS.is_valid_version("1.0.0"));
    }

    #[test]
    fn net_devices_require_v1_1_0() {
        let spec = spec_with_edits(
            "1.1.0",
            ContainerEdits {
                net_devices: Some(vec![LinuxNetDevice {
                    host_interface_name: "eth0".to_string(),
                    name: "container_eth0".to_string(),
                }]),
                ..Default::default()
            },
        );

        assert_eq!(
            minimum_required_version(&spec).unwrap().to_string(),
            "1.1.0"
        );
    }

    #[test]
    fn intel_rdt_schemata_requires_v1_1_0() {
        let spec = spec_with_edits(
            "1.1.0",
            ContainerEdits {
                intel_rdt: Some(IntelRdt {
                    schemata: Some(vec!["L3:0=ffff".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert_eq!(
            minimum_required_version(&spec).unwrap().to_string(),
            "1.1.0"
        );
    }

    #[test]
    fn empty_intel_rdt_schemata_requires_v1_1_0() {
        let global_spec = spec_with_edits(
            "1.1.0",
            ContainerEdits {
                intel_rdt: Some(IntelRdt {
                    schemata: Some(Vec::new()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        let device_spec = Spec {
            version: "1.1.0".to_string(),
            kind: "vendor.com/device".to_string(),
            devices: vec![Device {
                name: "gpu0".to_string(),
                container_edits: ContainerEdits {
                    intel_rdt: Some(IntelRdt {
                        schemata: Some(Vec::new()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_eq!(
            minimum_required_version(&global_spec).unwrap().to_string(),
            "1.1.0"
        );
        assert_eq!(
            minimum_required_version(&device_spec).unwrap().to_string(),
            "1.1.0"
        );
    }

    #[test]
    fn intel_rdt_enable_monitoring_requires_v1_1_0() {
        let spec = spec_with_edits(
            "1.1.0",
            ContainerEdits {
                intel_rdt: Some(IntelRdt {
                    enable_monitoring: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        assert_eq!(
            minimum_required_version(&spec).unwrap().to_string(),
            "1.1.0"
        );
    }
}
