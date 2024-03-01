
// CurrentVersion is the current version of the Spec.
const CURRENT_VERSION: &str = "0.7.0";

// Spec is the base configuration for CDI
#[derive(Serialize, Deserialize, Debug)]
pub struct Spec {
    #[serde(rename = "cdiVersion")]
    version: String,

    #[serde(rename = "kind")]
    kind: String,

    #[serde(rename = "annotations", default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    annotations: std::collections::HashMap<String, String>,

    #[serde(rename = "devices")]
    devices: Vec<Device>,

    #[serde(rename = "containerEdits", skip_serializing_if = "Option::is_none")]
    container_edits: Option<ContainerEdits>,
}

// Device is a "Device" a container runtime can add to a container
#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    #[serde(rename = "name")]
    name: String,

    #[serde(rename = "annotations", default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    annotations: std::collections::HashMap<String, String>,

    #[serde(rename = "containerEdits")]
    container_edits: ContainerEdits,
}

// ContainerEdits are edits a container runtime must make to the OCI spec to expose the device.
#[derive(Serialize, Deserialize, Debug)]
pub struct ContainerEdits {
    #[serde(rename = "env", default, skip_serializing_if = "Vec::is_empty")]
    env: Vec<String>,

    #[serde(rename = "deviceNodes", default, skip_serializing_if = "Vec::is_empty")]
    device_nodes: Vec<DeviceNode>,

    #[serde(rename = "hooks", default, skip_serializing_if = "Vec::is_empty")]
    hooks: Vec<Hook>,

    #[serde(rename = "mounts", default, skip_serializing_if = "Vec::is_empty")]
    mounts: Vec<Mount>,

    #[serde(rename = "intelRdt", skip_serializing_if = "Option::is_none")]
    intel_rdt: Option<IntelRdt>,

    #[serde(rename = "additionalGids", default, skip_serializing_if = "Vec::is_empty")]
    additional_gids: Vec<u32>,
}

// DeviceNode represents a device node that needs to be added to the OCI spec.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceNode {
    #[serde(rename = "path")]
    path: String,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_: Option<String>,

    #[serde(rename = "major", skip_serializing_if = "Option::is_none")]
    major: Option<i64>,

    #[serde(rename = "minor", skip_serializing_if = "Option::is_none")]
    minor: Option<i64>,

    #[serde(rename = "fileMode", skip_serializing_if = "Option::is_none")]
    file_mode: Option<PermissionsExt>,

    #[serde(rename = "permissions", skip_serializing_if = "Option::is_none")]
    permissions: Option<String>,

    #[serde(rename = "uid", skip_serializing_if = "Option::is_none")]
    uid: Option<u32>,

    #[serde(rename = "gid", skip_serializing_if = "Option::is_none")]
    gid: Option<u32>,
}

// Mount represents a mount that needs to be added to the OCI spec.
#[derive(Serialize, Deserialize, Debug)]
pub struct Mount {
    #[serde(rename = "hostPath")]
    host_path: String,

    #[serde(rename = "containerPath")]
    container_path: String,

    #[serde(rename = "options", default, skip_serializing_if = "Vec::is_empty")]
    options: Vec<String>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    mount_type: Option<String>,
}

// Hook represents a hook that needs to be added to the OCI spec.
#[derive(Serialize, Deserialize, Debug)]
pub struct Hook {
    #[serde(rename = "hookName")]
    hook_name: String,

    #[serde(rename = "path")]
    path: String,

    #[serde(rename = "args", default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,

    #[serde(rename = "env", default, skip_serializing_if = "Vec::is_empty")]
    env: Vec<String>,

    #[serde(rename = "timeout", skip_serializing_if = "Option::is_none")]
    timeout: Option<i32>,
}

// IntelRdt describes the Linux IntelRdt parameters to set in the OCI spec.
#[derive(Serialize, Deserialize, Debug)]
pub struct IntelRdt {
    #[serde(rename = "closID", skip_serializing_if = "Option::is_none")]
    clos_id: Option<String>,

    #[serde(rename = "l3CacheSchema", skip_serializing_if = "Option::is_none")]
    l3_cache_schema: Option<String>,

    #[serde(rename = "memBwSchema", skip_serializing_if = "Option::is_none")]
    mem_bw_schema: Option<String>,

    #[serde(rename = "enableCMT")]
    enable_cmt: bool,

    #[serde(rename = "enableMBM")]
    enable_mbm: bool,
}
