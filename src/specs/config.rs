use std::collections::BTreeMap;

use libc::mode_t;

use serde::{Deserialize, Serialize};

// CurrentVersion is the current version of the Spec.
#[allow(dead_code)]
const CURRENT_VERSION: &str = "0.7.0";

// Spec is the base configuration for CDI
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Spec {
    #[serde(rename = "cdiVersion")]
    pub(crate) version: String,

    #[serde(rename = "kind")]
    pub(crate) kind: String,

    #[serde(
        rename = "annotations",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub(crate) annotations: BTreeMap<String, String>,

    #[serde(rename = "devices")]
    pub(crate) devices: Vec<Device>,

    #[serde(rename = "containerEdits", skip_serializing_if = "Option::is_none")]
    pub(crate) container_edits: Option<ContainerEdits>,
}

// Device is a "Device" a container runtime can add to a container
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Device {
    #[serde(rename = "name")]
    pub(crate) name: String,

    #[serde(
        rename = "annotations",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub(crate) annotations: BTreeMap<String, String>,

    #[serde(rename = "containerEdits")]
    pub(crate) container_edits: ContainerEdits,
}

// ContainerEdits are edits a container runtime must make to the OCI spec to expose the device.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct ContainerEdits {
    #[serde(rename = "env", skip_serializing_if = "Option::is_none")]
    pub(crate) env: Option<Vec<String>>,

    #[serde(rename = "deviceNodes", skip_serializing_if = "Option::is_none")]
    pub(crate) device_nodes: Option<Vec<DeviceNode>>,

    #[serde(rename = "hooks", skip_serializing_if = "Option::is_none")]
    pub(crate) hooks: Option<Vec<Hook>>,

    #[serde(rename = "mounts", skip_serializing_if = "Option::is_none")]
    pub(crate) mounts: Option<Vec<Mount>>,

    #[serde(rename = "intelRdt", skip_serializing_if = "Option::is_none")]
    pub(crate) intel_rdt: Option<IntelRdt>,

    #[serde(rename = "additionalGids", skip_serializing_if = "Option::is_none")]
    pub(crate) additional_gids: Option<Vec<u32>>,
}

// DeviceNode represents a device node that needs to be added to the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct DeviceNode {
    #[serde(rename = "path")]
    pub(crate) path: String,

    #[serde(rename = "hostPath", skip_serializing_if = "Option::is_none")]
    pub(crate) host_path: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub(crate) r#type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minor: Option<i64>,

    #[serde(rename = "fileMode", skip_serializing_if = "Option::is_none")]
    pub file_mode: Option<mode_t>,

    #[serde(rename = "permissions", skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gid: Option<u32>,
}

// Mount represents a mount that needs to be added to the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Mount {
    #[serde(rename = "hostPath")]
    pub(crate) host_path: String,

    #[serde(rename = "containerPath")]
    pub(crate) container_path: String,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub(crate) r#type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<Vec<String>>,
}

// Hook represents a hook that needs to be added to the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Hook {
    #[serde(rename = "hookName")]
    pub(crate) hook_name: String,

    #[serde(rename = "path")]
    pub(crate) path: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) args: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) env: Option<Vec<String>>,

    #[serde(rename = "timeout", skip_serializing_if = "Option::is_none")]
    pub(crate) timeout: Option<i64>,
}

// IntelRdt describes the Linux IntelRdt parameters to set in the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct IntelRdt {
    #[serde(rename = "closID", skip_serializing_if = "Option::is_none")]
    pub(crate) clos_id: Option<String>,

    #[serde(rename = "l3CacheSchema", skip_serializing_if = "Option::is_none")]
    pub(crate) l3_cache_schema: Option<String>,

    #[serde(rename = "memBwSchema", skip_serializing_if = "Option::is_none")]
    pub(crate) mem_bw_schema: Option<String>,

    #[serde(default, rename = "enableCMT")]
    pub(crate) enable_cmt: bool,

    #[serde(default, rename = "enableMBM")]
    pub(crate) enable_mbm: bool,
}
