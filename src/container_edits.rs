use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context, Error, Result};
use oci_spec::runtime::{self as oci, LinuxDeviceType};

use crate::{
    container_edits_unix::{device_info_from_path, DeviceType},
    generate::config::Generator,
    specs::config::{
        ContainerEdits as CDIContainerEdits, DeviceNode as CDIDeviceNode, Hook as CDIHook,
        IntelRdt as CDIIntelRdt, LinuxNetDevice, Mount as CDIMount,
    },
    utils::merge,
};

const NO_PERMISSIONS: &str = "none";

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

fn validate_envs(envs: &[String]) -> Result<()> {
    if let Some(env) = envs
        .iter()
        .find(|v| v.split_once('=').is_none_or(|(name, _)| name.is_empty()))
    {
        return Err(anyhow!(
            "invalid environment variable {:?}: missing '=' or empty variable name",
            env
        ));
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
                if dev.uid().is_none() {
                    if let Some(process) = oci_spec.process() {
                        let uid = process.user().uid();
                        if uid > 0 {
                            dev.set_uid(Some(uid));
                        }
                    }
                }
                if dev.gid().is_none() {
                    if let Some(process) = oci_spec.process() {
                        let gid = process.user().gid();
                        if gid > 0 {
                            dev.set_gid(Some(gid));
                        }
                    }
                }

                let dev_typ = dev.typ();
                let typs = [LinuxDeviceType::B, LinuxDeviceType::C];
                if typs.contains(&dev_typ) {
                    let dev_access = match d.permissions.as_deref() {
                        None | Some("") => Some("rwm".to_string()),
                        Some(NO_PERMISSIONS) => Some(String::new()),
                        Some(permissions) => Some(permissions.to_string()),
                    };

                    let major = dev.major();
                    let minor = dev.minor();
                    spec_gen.add_linux_resources_device(
                        true,
                        dev_typ,
                        Some(major),
                        Some(minor),
                        dev_access,
                    );
                }

                spec_gen.remove_device(&dev.path().display().to_string());
                spec_gen.add_device(dev.clone());
            }
        }

        if let Some(net_devices) = &self.container_edits.net_devices {
            for net_device in net_devices {
                spec_gen.add_linux_net_device(
                    net_device.host_interface_name.clone(),
                    net_device.to_oci()?,
                );
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
            spec_gen.set_linux_intel_rdt(intel_rdt.to_oci()?);
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
        let intel_rdt = o
            .container_edits
            .intel_rdt
            .or_else(|| self.container_edits.intel_rdt.take());

        let ce = CDIContainerEdits {
            env: merge(&mut self.container_edits.env, &o.container_edits.env),
            device_nodes: merge(
                &mut self.container_edits.device_nodes,
                &o.container_edits.device_nodes,
            ),
            net_devices: merge(
                &mut self.container_edits.net_devices,
                &o.container_edits.net_devices,
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

    pub(crate) fn is_empty(&self) -> bool {
        option_vec_empty(&self.container_edits.env)
            && option_vec_empty(&self.container_edits.device_nodes)
            && option_vec_empty(&self.container_edits.net_devices)
            && option_vec_empty(&self.container_edits.hooks)
            && option_vec_empty(&self.container_edits.mounts)
            && self.container_edits.intel_rdt.is_none()
            && option_vec_empty(&self.container_edits.additional_gids)
    }
}

fn option_vec_empty<T>(value: &Option<Vec<T>>) -> bool {
    value.as_ref().is_none_or(Vec::is_empty)
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
        if let Some(net_devices) = &self.container_edits.net_devices {
            validate_net_devices(net_devices)?;
        }

        Ok(())
    }
}

fn validate_net_devices(devices: &[LinuxNetDevice]) -> Result<()> {
    let mut host_seen = HashSet::new();
    let mut name_seen = HashSet::new();

    for dev in devices {
        if dev.host_interface_name.is_empty() {
            return Err(anyhow!("invalid linux net device, empty HostInterfaceName"));
        }
        if dev.name.is_empty() {
            return Err(anyhow!("invalid linux net device, empty Name"));
        }
        if !host_seen.insert(dev.host_interface_name.clone()) {
            return Err(anyhow!(
                "invalid linux net device, duplicate HostInterfaceName {:?}",
                dev.host_interface_name
            ));
        }
        if !name_seen.insert(dev.name.clone()) {
            return Err(anyhow!(
                "invalid linux net device, duplicate Name {:?}",
                dev.name
            ));
        }
    }

    Ok(())
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

        if self.node.r#type.as_deref() == Some("") {
            self.node.r#type = None;
        }

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
            if !valid_typs.contains(typ.as_str()) {
                return Err(anyhow!(
                    "device {:?}: invalid type {:?}",
                    self.node.path,
                    typ
                ));
            }
        }

        if let Some(perms) = &self.node.permissions {
            match perms.as_str() {
                "" | NO_PERMISSIONS => {}
                _ if perms.chars().all(|c| matches!(c, 'r' | 'w' | 'm')) => {}
                _ => {
                    return Err(anyhow!(
                        "device {}: invalid permissions {}",
                        self.node.path,
                        perms
                    ));
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specs::config::{
        ContainerEdits as CDIContainerEdits, DeviceNode as CDIDeviceNode, IntelRdt, LinuxNetDevice,
    };
    use oci_spec::runtime::{Process, Spec};

    #[test]
    fn validates_env_requires_name_and_separator() {
        validate_envs(&["FOO=bar".to_string(), "EMPTY=".to_string()]).unwrap();

        let missing_separator = validate_envs(&["FOO".to_string()]).unwrap_err();
        assert!(missing_separator.to_string().contains("missing '='"));

        let empty_name = validate_envs(&["=bar".to_string()]).unwrap_err();
        assert!(empty_name.to_string().contains("empty variable name"));
    }

    #[test]
    fn validates_device_type_and_permissions() {
        let valid = CDIDeviceNode {
            path: "/dev/example".to_string(),
            r#type: Some("c".to_string()),
            permissions: Some("none".to_string()),
            ..Default::default()
        };
        assert!(DeviceNode { node: valid }.validate().is_ok());

        let invalid_type = CDIDeviceNode {
            path: "/dev/example".to_string(),
            r#type: Some("bad".to_string()),
            ..Default::default()
        };
        assert!(DeviceNode { node: invalid_type }.validate().is_err());
    }

    #[test]
    fn fill_missing_info_treats_empty_device_type_as_unset() {
        let mut device = DeviceNode {
            node: CDIDeviceNode {
                path: "/dev/null".to_string(),
                r#type: Some(String::new()),
                ..Default::default()
            },
        };

        device.fill_missing_info().unwrap();

        assert_eq!(Some("c"), device.node.r#type.as_deref());
        assert_eq!(Some(1), device.node.major);
        assert_eq!(Some(3), device.node.minor);
    }

    #[test]
    fn validates_net_device_duplicates() {
        let edits = ContainerEdits {
            container_edits: CDIContainerEdits {
                net_devices: Some(vec![
                    LinuxNetDevice {
                        host_interface_name: "eth0".to_string(),
                        name: "container_eth0".to_string(),
                    },
                    LinuxNetDevice {
                        host_interface_name: "eth0".to_string(),
                        name: "container_eth1".to_string(),
                    },
                ]),
                ..Default::default()
            },
        };

        assert!(edits.validate().is_err());
    }

    #[test]
    fn apply_sets_net_devices_and_full_intel_rdt() {
        let mut spec = Spec::default();
        let mut edits = ContainerEdits {
            container_edits: CDIContainerEdits {
                net_devices: Some(vec![LinuxNetDevice {
                    host_interface_name: "eth0".to_string(),
                    name: "container_eth0".to_string(),
                }]),
                intel_rdt: Some(IntelRdt {
                    clos_id: Some("class-a".to_string()),
                    schemata: Some(vec!["L3:0=ffff".to_string()]),
                    enable_monitoring: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        };

        edits.apply(&mut spec).unwrap();

        let linux = spec.linux().as_ref().unwrap();
        assert_eq!(
            linux
                .net_devices()
                .as_ref()
                .unwrap()
                .get("eth0")
                .unwrap()
                .name()
                .as_ref()
                .unwrap(),
            "container_eth0"
        );
        let rdt = linux.intel_rdt().as_ref().unwrap();
        assert_eq!(Some(&"class-a".to_string()), rdt.clos_id().as_ref());
        assert_eq!(
            Some(&vec!["L3:0=ffff".to_string()]),
            rdt.schemata().as_ref()
        );
        assert_eq!(Some(true), *rdt.enable_monitoring());
    }

    #[test]
    fn apply_uses_process_uid_and_gid_only_when_device_owner_unset() {
        let mut spec = Spec::default();
        spec.set_process(Some(Process::default()));
        spec.process_mut()
            .as_mut()
            .unwrap()
            .user_mut()
            .set_uid(1234);
        spec.process_mut()
            .as_mut()
            .unwrap()
            .user_mut()
            .set_gid(5678);

        let mut edits = ContainerEdits {
            container_edits: CDIContainerEdits {
                device_nodes: Some(vec![CDIDeviceNode {
                    path: "/dev/null".to_string(),
                    r#type: Some("c".to_string()),
                    major: Some(1),
                    minor: Some(3),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        };

        edits.apply(&mut spec).unwrap();
        let dev = &spec.linux().as_ref().unwrap().devices().as_ref().unwrap()[0];
        assert_eq!(Some(1234), dev.uid());
        assert_eq!(Some(5678), dev.gid());
    }
}
