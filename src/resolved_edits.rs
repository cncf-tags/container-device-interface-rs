use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

use crate::{
    container_edits::{ContainerEdits, DeviceNode},
    container_edits_unix::{device_info_from_path, DeviceType},
    specs::config::{DeviceNode as CDIDeviceNode, Mount as CDIMount},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedCdiEdits {
    pub env: Vec<String>,
    pub device_nodes: Vec<ResolvedCdiDeviceNode>,
    pub mounts: Vec<ResolvedCdiMount>,
    pub unsupported: Vec<UnsupportedCdiEdit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedCdiDeviceNode {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub typ: Option<String>,
    pub major: Option<i64>,
    pub minor: Option<i64>,
    pub file_mode: Option<libc::mode_t>,
    pub permissions: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub scope: CdiEditScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedCdiMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub typ: Option<String>,
    pub options: Vec<String>,
    pub scope: CdiEditScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedCdiEdit {
    pub kind: UnsupportedCdiEditKind,
    pub count: usize,
    pub scope: CdiEditScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsupportedCdiEditKind {
    Hooks,
    NetDevices,
    IntelRdt,
    AdditionalGids,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdiEditScope {
    Spec {
        kind: String,
        path: PathBuf,
    },
    Device {
        qualified_name: String,
        spec_kind: String,
        spec_path: PathBuf,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ScopedContainerEdits {
    pub(crate) edits: ContainerEdits,
    pub(crate) scope: CdiEditScope,
}

impl ResolvedCdiEdits {
    pub(crate) fn append_container_edits(
        &mut self,
        edits: &ContainerEdits,
        scope: CdiEditScope,
    ) -> Result<()> {
        let edits = &edits.container_edits;

        if let Some(env) = &edits.env {
            self.env.extend(env.iter().cloned());
        }

        if let Some(device_nodes) = &edits.device_nodes {
            for device_node in device_nodes {
                self.device_nodes
                    .push(resolve_device_node(device_node, scope.clone())?);
            }
        }

        if let Some(mounts) = &edits.mounts {
            for mount in mounts {
                self.mounts.push(resolve_mount(mount, scope.clone()));
            }
        }

        if let Some(hooks) = &edits.hooks {
            self.report_unsupported(UnsupportedCdiEditKind::Hooks, hooks.len(), scope.clone());
        }
        if let Some(net_devices) = &edits.net_devices {
            self.report_unsupported(
                UnsupportedCdiEditKind::NetDevices,
                net_devices.len(),
                scope.clone(),
            );
        }
        if edits.intel_rdt.is_some() {
            self.report_unsupported(UnsupportedCdiEditKind::IntelRdt, 1, scope.clone());
        }
        if let Some(additional_gids) = &edits.additional_gids {
            self.report_unsupported(
                UnsupportedCdiEditKind::AdditionalGids,
                additional_gids.len(),
                scope,
            );
        }

        Ok(())
    }

    fn report_unsupported(
        &mut self,
        kind: UnsupportedCdiEditKind,
        count: usize,
        scope: CdiEditScope,
    ) {
        if count > 0 {
            self.unsupported
                .push(UnsupportedCdiEdit { kind, count, scope });
        }
    }
}

fn resolve_device_node(node: &CDIDeviceNode, scope: CdiEditScope) -> Result<ResolvedCdiDeviceNode> {
    let is_unbuffered_char = node.r#type.as_deref() == Some("u");
    let mut node_for_fill = node.clone();
    if is_unbuffered_char {
        node_for_fill.r#type = Some(DeviceType::Char.to_string());
    }

    let mut device_node = DeviceNode {
        node: node_for_fill,
    };
    device_node
        .fill_missing_info()
        .with_context(|| format!("failed to resolve CDI device node {}", node.path))?;

    let mut node = device_node.node;
    if is_unbuffered_char {
        node.r#type = Some("u".to_string());
    }

    let host_path = node.host_path.clone().unwrap_or_else(|| node.path.clone());
    let (host_type, host_major, host_minor) = device_info_from_path(&host_path)
        .with_context(|| format!("failed to inspect CDI device node host path {host_path}"))?;

    match node.r#type.as_deref() {
        None => node.r#type = Some(host_type.clone()),
        Some(node_type) if !device_types_match(node_type, host_type.as_str()) => {
            return Err(anyhow!(
                "CDI device ({}, {}), host type mismatch ({}, {})",
                node.path,
                host_path,
                node_type,
                host_type
            ));
        }
        _ => {}
    }

    let fifo_type = DeviceType::Fifo.to_string();
    if node.r#type.as_deref() != Some(fifo_type.as_str()) {
        if node.major.is_none() {
            node.major = Some(host_major);
        }
        if node.minor.is_none() {
            node.minor = Some(host_minor);
        }
    }

    Ok(ResolvedCdiDeviceNode {
        host_path: PathBuf::from(host_path),
        container_path: PathBuf::from(node.path),
        typ: node.r#type.filter(|typ| !typ.is_empty()),
        major: node.major,
        minor: node.minor,
        file_mode: node.file_mode,
        permissions: node.permissions,
        uid: node.uid,
        gid: node.gid,
        scope,
    })
}

fn device_types_match(node_type: &str, host_type: &str) -> bool {
    node_type == host_type || matches!((node_type, host_type), ("u", "c"))
}

fn resolve_mount(mount: &CDIMount, scope: CdiEditScope) -> ResolvedCdiMount {
    ResolvedCdiMount {
        host_path: PathBuf::from(&mount.host_path),
        container_path: PathBuf::from(&mount.container_path),
        typ: mount.r#type.clone(),
        options: mount.options.clone().unwrap_or_default(),
        scope,
    }
}
