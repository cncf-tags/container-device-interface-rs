use std::collections::BTreeMap;

use libc::mode_t;

use serde::{Deserialize, Serialize};

// CurrentVersion is the current version of the Spec.
pub const CURRENT_VERSION: &str = "1.1.0";

// Spec is the base configuration for CDI
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(rename = "cdiVersion")]
    pub version: String,

    #[serde(rename = "kind")]
    pub kind: String,

    #[serde(
        rename = "annotations",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub annotations: BTreeMap<String, String>,

    #[serde(rename = "devices")]
    pub devices: Vec<Device>,

    #[serde(rename = "containerEdits", skip_serializing_if = "Option::is_none")]
    pub container_edits: Option<ContainerEdits>,
}

// Device is a "Device" a container runtime can add to a container
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    #[serde(rename = "name")]
    pub name: String,

    #[serde(
        rename = "annotations",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub annotations: BTreeMap<String, String>,

    #[serde(rename = "containerEdits")]
    pub container_edits: ContainerEdits,
}

// ContainerEdits are edits a container runtime must make to the OCI spec to expose the device.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerEdits {
    #[serde(rename = "env", skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,

    #[serde(rename = "deviceNodes", skip_serializing_if = "Option::is_none")]
    pub device_nodes: Option<Vec<DeviceNode>>,

    #[serde(rename = "netDevices", skip_serializing_if = "Option::is_none")]
    pub net_devices: Option<Vec<LinuxNetDevice>>,

    #[serde(rename = "hooks", skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Vec<Hook>>,

    #[serde(rename = "mounts", skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<Mount>>,

    #[serde(rename = "intelRdt", skip_serializing_if = "Option::is_none")]
    pub intel_rdt: Option<IntelRdt>,

    #[serde(rename = "additionalGids", skip_serializing_if = "Option::is_none")]
    pub additional_gids: Option<Vec<u32>>,
}

// DeviceNode represents a device node that needs to be added to the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceNode {
    #[serde(rename = "path")]
    pub path: String,

    #[serde(rename = "hostPath", skip_serializing_if = "Option::is_none")]
    pub host_path: Option<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

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
#[serde(deny_unknown_fields)]
pub struct Mount {
    #[serde(rename = "hostPath")]
    pub host_path: String,

    #[serde(rename = "containerPath")]
    pub container_path: String,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

// Hook represents a hook that needs to be added to the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    #[serde(rename = "hookName")]
    pub hook_name: String,

    #[serde(rename = "path")]
    pub path: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,

    #[serde(rename = "timeout", skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i64>,
}

// IntelRdt describes the Linux IntelRdt parameters to set in the OCI spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntelRdt {
    #[serde(rename = "closID", skip_serializing_if = "Option::is_none")]
    pub clos_id: Option<String>,

    #[serde(rename = "l3CacheSchema", skip_serializing_if = "Option::is_none")]
    pub l3_cache_schema: Option<String>,

    #[serde(rename = "memBwSchema", skip_serializing_if = "Option::is_none")]
    pub mem_bw_schema: Option<String>,

    #[serde(rename = "schemata", skip_serializing_if = "Option::is_none")]
    pub schemata: Option<Vec<String>>,

    #[serde(rename = "enableMonitoring", skip_serializing_if = "Option::is_none")]
    pub enable_monitoring: Option<bool>,

    #[serde(rename = "enableCMT", skip_serializing_if = "Option::is_none")]
    pub enable_cmt: Option<bool>,

    #[serde(rename = "enableMBM", skip_serializing_if = "Option::is_none")]
    pub enable_mbm: Option<bool>,
}

// LinuxNetDevice represents an OCI LinuxNetDevice to be added to the OCI Spec.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinuxNetDevice {
    #[serde(rename = "hostInterfaceName")]
    pub host_interface_name: String,

    #[serde(rename = "name")]
    pub name: String,
}
