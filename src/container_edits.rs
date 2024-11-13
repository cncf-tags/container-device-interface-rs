use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context, Error, Result};
use oci_spec::runtime::{self as oci, LinuxDeviceType};

use crate::{
    container_edits_unix::{device_info_from_path, DeviceType},
    generate::config::Generator,
    specs::config::{
        ContainerEdits as CDIContainerEdits, DeviceNode as CDIDeviceNode, Hook as CDIHook,
        IntelRdt as CDIIntelRdt, Mount as CDIMount,
    },
    utils::merge,
};

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

fn validate_envs(envs: &[String]) -> Result<()> {
    if envs.iter().any(|v| !v.contains('=')) {
        return Err(anyhow!("invalid environment variable: {:?}", envs));
    }

    Ok(())
}

// ContainerEdits represent updates to be applied to an OCI Spec.
// These updates can be specific to a CDI device, or they can be
// specific to a CDI Spec. In the former case these edits should
// be applied to all OCI Specs where the corresponding CDI device
// is injected. In the latter case, these edits should be applied
// to all OCI Specs where at least one devices from the CDI Spec
// is injected.
#[derive(Clone, Default, Debug, Eq, PartialEq, Hash)]
pub struct ContainerEdits {
    pub container_edits: CDIContainerEdits,
}

impl ContainerEdits {
    pub fn new() -> Self {
        Self {
            container_edits: CDIContainerEdits {
                ..Default::default()
            },
        }
    }

    // apply edits to the given OCI Spec. Updates the OCI Spec in place.
    // Returns an error if the update fails.
    pub fn apply(&mut self, oci_spec: &mut oci::Spec) -> Result<()> {
        let mut spec_gen: Generator = Generator::spec_gen(Some(oci_spec.clone()));

        if let Some(envs) = &self.container_edits.env {
            if !envs.is_empty() {
                spec_gen.add_multiple_process_env(envs);
            }
        }

        if let Some(device_nodes) = &self.container_edits.device_nodes {
            for d in device_nodes {
                let mut dn: DeviceNode = DeviceNode { node: d.clone() };

                dn.fill_missing_info()
                    .context("filling missing info failed.")?;

                let d = &dn.node;
                let mut dev = dn.node.to_oci()?;
                if let Some(process) = oci_spec.process_mut() {
                    let user = process.user_mut();
                    let gid = user.gid();
                    if gid > 0 {
                        dev.set_gid(Some(gid));
                    }

                    let uid = user.uid();
                    if uid > 0 {
                        dev.set_gid(Some(uid));
                    }
                }

                let dev_typ = dev.typ();
                let typs = [LinuxDeviceType::B, LinuxDeviceType::C];
                if typs.contains(&dev_typ) {
                    let perms = "rwm".to_owned();
                    let dev_access = if let Some(permissions) = &d.permissions {
                        permissions
                    } else {
                        &perms
                    };

                    let major = dev.major();
                    let minor = dev.minor();
                    spec_gen.add_linux_resources_device(
                        true,
                        dev_typ,
                        Some(major),
                        Some(minor),
                        Some(dev_access.clone()),
                    );
                }

                spec_gen.remove_device(&dev.path().display().to_string());
                spec_gen.add_device(dev.clone());
            }
        }

        if let Some(mounts) = &self.container_edits.mounts {
            for m in mounts {
                spec_gen.remove_mount(&m.container_path);
                spec_gen.add_mount(m.to_oci()?);
            }
        }

        if let Some(hooks) = &self.container_edits.hooks {
            for h in hooks {
                let hook_name = HookName::from_str(&h.hook_name)
                    .context(format!("no such hook with name: {:?}", &h.hook_name))?;
                match hook_name {
                    HookName::Prestart => spec_gen.add_prestart_hook(h.to_oci()?),
                    HookName::CreateRuntime => spec_gen.add_createruntime_hook(h.to_oci()?),
                    HookName::CreateContainer => spec_gen.add_createcontainer_hook(h.to_oci()?),
                    HookName::StartContainer => spec_gen.add_startcontainer_hook(h.to_oci()?),
                    HookName::Poststart => spec_gen.add_poststart_hook(h.to_oci()?),
                    HookName::Poststop => spec_gen.add_poststop_hook(h.to_oci()?),
                }
            }
        }

        if let Some(intel_rdt) = &self.container_edits.intel_rdt {
            if let Some(clos_id) = &intel_rdt.clos_id {
                spec_gen.set_linux_intel_rdt_clos_id(clos_id.to_string());
                // TODO: spec.Linux.IntelRdt = e.IntelRdt.ToOCI()
            }
        }

        if let Some(additional_gids) = &self.container_edits.additional_gids {
            for gid in additional_gids {
                if *gid > 0 {
                    spec_gen.add_process_additional_gid(*gid);
                }
            }
        }

        if let Some(ref spec) = spec_gen.config {
            oci_spec.set_linux(spec.linux().clone());
            oci_spec.set_mounts(spec.mounts().clone());
            oci_spec.set_annotations(spec.annotations().clone());
            oci_spec.set_hooks(spec.hooks().clone());
            oci_spec.set_process(spec.process().clone());
        }

        Ok(())
    }

    // append other edits into this one.
    pub fn append(&mut self, o: ContainerEdits) -> Result<()> {
        let intel_rdt = if o.container_edits.intel_rdt.is_some() {
            o.container_edits.intel_rdt
        } else {
            None
        };

        let ce = CDIContainerEdits {
            env: merge(&mut self.container_edits.env, &o.container_edits.env),
            device_nodes: merge(
                &mut self.container_edits.device_nodes,
                &o.container_edits.device_nodes,
            ),
            hooks: merge(&mut self.container_edits.hooks, &o.container_edits.hooks),
            mounts: merge(&mut self.container_edits.mounts, &o.container_edits.mounts),
            intel_rdt,
            additional_gids: merge(
                &mut self.container_edits.additional_gids,
                &o.container_edits.additional_gids,
            ),
        };

        self.container_edits = ce;

        Ok(())
    }
}

// Validate container edits.
impl Validate for ContainerEdits {
    fn validate(&self) -> Result<()> {
        if let Some(envs) = &self.container_edits.env {
            validate_envs(envs)
                .context(format!("invalid container edits with envs: {:?}", envs))?;
        }
        if let Some(devices) = &self.container_edits.device_nodes {
            for d in devices {
                let dn = DeviceNode { node: d.clone() };
                dn.validate()
                    .context(format!("invalid container edits with device: {:?}", &d))?;
            }
        }
        if let Some(hooks) = &self.container_edits.hooks {
            for h in hooks {
                let hook = Hook { hook: h.clone() };
                hook.validate()
                    .context(format!("invalid container edits with hook: {:?}", &h))?;
            }
        }
        if let Some(mounts) = &self.container_edits.mounts {
            for m in mounts {
                let mnt = Mount { mount: m.clone() };
                mnt.validate()
                    .context(format!("invalid container edits with mount: {:?}", &m))?;
            }
        }
        if let Some(irdt) = &self.container_edits.intel_rdt {
            let i_rdt = IntelRdt {
                intel_rdt: irdt.clone(),
            };
            i_rdt
                .validate()
                .context(format!("invalid container edits with mount: {:?}", irdt))?;
        }

        Ok(())
    }
}

// DeviceNode is a CDI Spec DeviceNode wrapper, used for validating DeviceNodes.
pub struct DeviceNode {
    pub node: CDIDeviceNode,
}

impl DeviceNode {
    pub fn fill_missing_info(&mut self) -> Result<()> {
        let host_path = self
            .node
            .host_path
            .as_deref()
            .unwrap_or_else(|| &self.node.path);

        if let Some(device_type) = self.node.r#type.as_deref() {
            if self.node.major.is_some() || device_type == DeviceType::Fifo.to_string() {
                return Ok(());
            }
        }

        let (dev_type, major, minor) = device_info_from_path(host_path)?;
        match self.node.r#type.as_deref() {
            None => self.node.r#type = Some(dev_type),
            Some(node_type) if node_type != dev_type => {
                return Err(anyhow!(
                    "CDI device ({}, {}), host type mismatch ({}, {})",
                    self.node.path,
                    host_path,
                    node_type,
                    dev_type
                ));
            }
            _ => {}
        }

        if self.node.major.is_none()
            && self.node.r#type.as_deref() != Some(&DeviceType::Fifo.to_string())
        {
            self.node.major = Some(major);
            self.node.minor = Some(minor);
        }

        Ok(())
    }
}

impl Validate for DeviceNode {
    fn validate(&self) -> Result<()> {
        let typs = vec!["b", "c", "u", "p", ""];
        let valid_typs: HashSet<&str> = typs.into_iter().collect();

        if self.node.path.is_empty() {
            return Err(anyhow!("invalid (empty) device path"));
        }

        if let Some(typ) = &self.node.r#type {
            if !valid_typs.contains(&typ.as_str()) {
                return Err(anyhow!(
                    "device {:?}: invalid type {:?}",
                    self.node.path,
                    typ
                ));
            }
        }

        if let Some(perms) = &self.node.permissions {
            if !perms.chars().all(|c| matches!(c, 'r' | 'w' | 'm')) {
                return Err(anyhow!(
                    "device {}: invalid permissions {}",
                    self.node.path,
                    perms
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum HookName {
    Prestart,
    CreateRuntime,
    CreateContainer,
    StartContainer,
    Poststart,
    Poststop,
}

impl FromStr for HookName {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "prestart" => Ok(Self::Prestart),
            "createRuntime" => Ok(Self::CreateRuntime),
            "createContainer" => Ok(Self::CreateContainer),
            "startContainer" => Ok(Self::StartContainer),
            "poststart" => Ok(Self::Poststart),
            "poststop" => Ok(Self::Poststop),
            _ => Err(anyhow!("no such hook")),
        }
    }
}

struct Hook {
    hook: CDIHook,
}

impl Validate for Hook {
    fn validate(&self) -> Result<()> {
        HookName::from_str(&self.hook.hook_name)
            .context(anyhow!("invalid hook name: {:?}", self.hook.hook_name))?;

        if self.hook.path.is_empty() {
            return Err(anyhow!(
                "invalid hook {:?} with empty path",
                self.hook.hook_name
            ));
        }
        if let Some(envs) = &self.hook.env {
            validate_envs(envs)
                .context(anyhow!("hook {:?} with invalid env", &self.hook.hook_name))?;
        }

        Ok(())
    }
}

struct Mount {
    mount: CDIMount,
}

impl Validate for Mount {
    fn validate(&self) -> Result<()> {
        if self.mount.host_path.is_empty() {
            return Err(anyhow!("invalid mount, empty host path"));
        }

        if self.mount.container_path.is_empty() {
            return Err(anyhow!("invalid mount, empty container path"));
        }

        Ok(())
    }
}

struct IntelRdt {
    intel_rdt: CDIIntelRdt,
}

impl Validate for IntelRdt {
    fn validate(&self) -> Result<()> {
        if let Some(ref clos_id) = self.intel_rdt.clos_id {
            if clos_id.len() >= 4096
                || clos_id == "."
                || clos_id == ".."
                || clos_id.contains(&['/', '\n'][..])
            {
                return Err(anyhow!("invalid clos id".to_string()));
            }
        }

        Ok(())
    }
}
