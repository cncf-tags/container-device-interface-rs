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

fn requires_v070(spec: &CDISpec) -> bool {
    let edits = &spec.container_edits;
    if let Some(edits) = edits {
        if edits.intel_rdt.as_ref().is_some() {
            return true;
        }
        if edits
            .additional_gids
            .as_ref()
            .map_or(false, |v| !v.is_empty())
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
            .map_or(false, |v| !v.is_empty())
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
                .map_or(false, |path| !path.is_empty())
        })
}

fn requires_v040(spec: &CDISpec) -> bool {
    spec.devices
        .iter()
        .map(|d| &d.container_edits)
        .chain(std::iter::once(spec.container_edits.as_ref().unwrap()))
        .flat_map(|edits| edits.mounts.iter().flat_map(|mounts| mounts.iter()))
        .any(|mount| mount.r#type.as_ref().map_or(false, |typ| !typ.is_empty()))
}
