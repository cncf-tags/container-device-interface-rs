use std::cmp::Ordering;
use std::path::PathBuf;

use oci_spec::runtime::{Hook, LinuxDevice, LinuxDeviceCgroup, LinuxDeviceType, Mount};

use super::config::Generator;

impl Generator {
    // remove_device removes a device from g.config.linux.devices
    pub fn remove_device(&mut self, path: &str) {
        self.init_config_linux();
        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if let Some(devices) = linux.devices_mut() {
                if let Some(index) = devices
                    .iter()
                    .position(|device| device.path() == &PathBuf::from(path))
                {
                    devices.remove(index);
                }
            }
        }
    }

    // add_device adds a device into g.config.linux.devices
    pub fn add_device(&mut self, device: LinuxDevice) {
        self.init_config_linux();

        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if let Some(devices) = linux.devices_mut() {
                if let Some(index) = devices.iter().position(|dev| dev.path() == device.path()) {
                    devices[index] = device;
                } else {
                    devices.push(device);
                }
            } else {
                linux.set_devices(Some(vec![device]));
            }
        }
    }

    // add_linux_resources_device adds a device into g.config.linux.resources.devices
    pub fn add_linux_resources_device(
        &mut self,
        allow: bool,
        dev_type: LinuxDeviceType,
        major: Option<i64>,
        minor: Option<i64>,
        access: Option<String>,
    ) {
        self.init_config_linux_resources_devices();
        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if let Some(resource) = linux.resources_mut() {
                if let Some(devices) = resource.devices_mut() {
                    let mut device = LinuxDeviceCgroup::default();
                    device.set_allow(allow);
                    device.set_typ(Some(dev_type));
                    device.set_major(major);
                    device.set_minor(minor);
                    device.set_access(access);

                    devices.push(device);
                }
            }
        }
    }

    /// set Linux Intel RDT ClosID
    pub fn set_linux_intel_rdt_clos_id(&mut self, clos_id: String) {
        self.init_config_linux_intel_rdt();
        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if let Some(intel_rdt) = linux.intel_rdt_mut() {
                intel_rdt.set_clos_id(Some(clos_id));
            }
        }
    }

    // add_process_additional_gid adds an additional gid into g.config.process.additional_gids.
    pub fn add_process_additional_gid(&mut self, gid: u32) {
        self.init_config_process();
        if let Some(process) = self.config.as_mut().unwrap().process_mut() {
            if let Some(additional_gids) = process.user().additional_gids() {
                let mut tmp_vec = additional_gids.clone();
                if !additional_gids.contains(&gid) {
                    tmp_vec.push(gid)
                }

                process.user_mut().set_additional_gids(Some(tmp_vec));
            }
        }
    }

    pub fn add_multiple_process_env(&mut self, envs: &[String]) {
        self.init_config_process();

        if let Some(process) = self.config.as_mut().unwrap().process_mut() {
            let mut env_vec: Vec<String> = process.env_mut().get_or_insert_with(Vec::new).to_vec();
            for env in envs {
                let split: Vec<&str> = env.splitn(2, '=').collect();
                let key = split[0].to_string();
                let idx = self.env_map.entry(key.clone()).or_insert(env_vec.len());

                if let Some(elem) = env_vec.get_mut(*idx) {
                    elem.clone_from(env);
                } else {
                    env_vec.push(env.clone());
                    self.env_map.insert(key, env_vec.len() - 1);
                }
            }
            process.set_env(Some(env_vec));
        }
    }

    // add_prestart_hook adds a prestart hook into g.config.hooks.prestart.
    pub fn add_prestart_hook(&mut self, hook: Hook) {
        self.init_config_hooks();
        if let Some(hooks) = self.config.as_mut().unwrap().hooks_mut() {
            if let Some(prestart_hooks) = hooks.prestart_mut() {
                prestart_hooks.push(hook);
            } else {
                hooks.set_prestart(Some(vec![hook]));
            }
        }
    }

    // add_poststop_hook adds a poststop hook into g.config.hooks.poststop.
    pub fn add_poststop_hook(&mut self, hook: Hook) {
        self.init_config_hooks();
        if let Some(hooks) = self.config.as_mut().unwrap().hooks_mut() {
            if let Some(poststop_hooks) = hooks.poststop_mut() {
                poststop_hooks.push(hook);
            } else {
                hooks.set_poststop(Some(vec![hook]));
            }
        }
    }

    // add_poststart_hook adds a poststart hook into g.config.hooks.poststart.
    pub fn add_poststart_hook(&mut self, hook: Hook) {
        self.init_config_hooks();
        if let Some(hooks) = self.config.as_mut().unwrap().hooks_mut() {
            if let Some(poststart_hooks) = hooks.poststart_mut() {
                poststart_hooks.push(hook);
            } else {
                hooks.set_poststart(Some(vec![hook]));
            }
        }
    }

    // add_createruntime_hook adds a create_runtime hook into g.config.hooks.create_runtime.
    pub fn add_createruntime_hook(&mut self, hook: Hook) {
        self.init_config_hooks();
        if let Some(hooks) = self.config.as_mut().unwrap().hooks_mut() {
            if let Some(create_runtime) = hooks.create_runtime_mut() {
                create_runtime.push(hook);
            } else {
                hooks.set_create_runtime(Some(vec![hook]));
            }
        }
    }

    // add_createcontainer_hook adds a create_container hook into g.config.hooks.create_container.
    pub fn add_createcontainer_hook(&mut self, hook: Hook) {
        self.init_config_hooks();
        if let Some(hooks) = self.config.as_mut().unwrap().hooks_mut() {
            if let Some(create_container) = hooks.create_container_mut() {
                create_container.push(hook);
            } else {
                hooks.set_create_container(Some(vec![hook]));
            }
        }
    }

    // add_start_container_hook adds a start container hook into g.config.hooks.start_container.
    pub fn add_startcontainer_hook(&mut self, hook: Hook) {
        self.init_config_hooks();
        if let Some(hooks) = self.config.as_mut().unwrap().hooks_mut() {
            if let Some(start_container) = hooks.start_container_mut() {
                start_container.push(hook);
            } else {
                hooks.set_start_container(Some(vec![hook]));
            }
        }
    }

    // remove_mount removes a mount point on the dest directory
    pub fn remove_mount(&mut self, dest: &str) {
        if let Some(mounts) = self.config.as_mut().unwrap().mounts_mut() {
            if let Some(index) = mounts
                .iter()
                .position(|m| m.destination() == &PathBuf::from(dest))
            {
                mounts.remove(index);
            }
        }
    }

    // add_mount adds a mount into g.config.mounts.
    pub fn add_mount(&mut self, mount: Mount) {
        self.init_config_mounts();

        if let Some(mounts) = self.config.as_mut().unwrap().mounts_mut() {
            mounts.push(mount);
        }
    }

    // sort_mounts sorts the mounts in the given OCI Spec.
    pub fn sort_mounts(&mut self) {
        if let Some(ref mut mounts) = self.config.as_mut().unwrap().mounts_mut() {
            mounts.sort_by(|a, b| a.destination().cmp(b.destination()));
        }
    }

    // list_mounts returns the list of mounts
    pub fn list_mounts(&self) -> Option<&Vec<Mount>> {
        self.config.as_ref().and_then(|spec| spec.mounts().as_ref())
    }

    // clear_mounts clear g.Config.Mounts
    pub fn clear_mounts(&mut self) {
        if let Some(spec) = self.config.as_mut() {
            spec.set_mounts(None);
        }
    }
}

// OrderedMounts defines how to sort an OCI Spec Mount slice.
// This is the almost the same implementation sa used by CRI-O and Docker,
// with a minor tweak for stable sorting order (easier to test):
//
//	https://github.com/moby/moby/blob/17.05.x/daemon/volumes.go#L26
struct OrderedMounts(Vec<Mount>);

#[allow(dead_code)]
impl OrderedMounts {
    fn new(mounts: Vec<Mount>) -> Self {
        OrderedMounts(mounts)
    }

    // parts returns the number of parts in the destination of a mount. Used in sorting.
    fn parts(&self, i: usize) -> usize {
        self.0[i].destination().components().count()
    }
}

impl Ord for OrderedMounts {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_parts = self.parts(0);
        let other_parts = other.parts(0);
        self_parts
            .cmp(&other_parts)
            .then_with(|| self.0[0].destination().cmp(other.0[0].destination()))
    }
}

impl PartialOrd for OrderedMounts {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for OrderedMounts {
    fn eq(&self, other: &Self) -> bool {
        self.parts(0) == other.parts(0) && self.0[0].destination() == other.0[0].destination()
    }
}

impl Eq for OrderedMounts {}
