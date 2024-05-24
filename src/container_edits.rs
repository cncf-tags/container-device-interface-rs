use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context, Error, Result};
use oci_spec::runtime as oci;

use crate::{
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
    // Apply edits to the given OCI Spec. Updates the OCI Spec in place.
    // Returns an error if the update fails.
    pub fn new() -> Self {
        Self {
            container_edits: CDIContainerEdits {
                ..Default::default()
            },
        }
    }

    pub fn apply(&mut self, _oci_spec: &mut oci::Spec) -> Result<()> {
        // TODO: it depends on Generator related to oci spec, however, there's no existing
        // Generator for us and the Generator depends on the MutGetters attribute on oci-spec-rs/runtime.
        // It will be implemented once the PR https://github.com/containers/oci-spec-rs/pull/166 merged
        Ok(())
    }

    pub fn append(&mut self, o: ContainerEdits) -> Self {
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

        Self {
            container_edits: ce,
        }
    }
}

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

impl Validate for DeviceNode {
    fn validate(&self) -> Result<()> {
        let typs = vec!["b", "c", "u", "p", ""];
        let valid_typs: HashSet<&str> = typs.into_iter().collect();

        if self.node.path.is_empty() {
            return Err(anyhow!("invalid (empty) device path"));
        }

        if let Some(typ) = &self.node.r#type {
            if valid_typs.contains(&typ.as_str()) {
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
                || clos_id.contains(|c| c == '/' || c == '\n')
            {
                return Err(anyhow!("invalid clos id".to_string()));
            }
        }

        Ok(())
    }
}
