use std::cmp::Ordering;
use std::path::PathBuf;

use oci_spec::runtime::{
    Hook, LinuxDevice, LinuxDeviceCgroup, LinuxDeviceType, LinuxIntelRdt, LinuxNetDevice, Mount,
};

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

    pub fn add_linux_net_device(
        &mut self,
        host_interface_name: String,
        net_device: LinuxNetDevice,
    ) {
        self.init_config_linux_net_devices();
        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if let Some(net_devices) = linux.net_devices_mut() {
                net_devices.insert(host_interface_name, net_device);
            }
        }
    }

    pub fn set_linux_intel_rdt(&mut self, intel_rdt: LinuxIntelRdt) {
        self.init_config_linux();
        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            linux.set_intel_rdt(Some(intel_rdt));
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
            let mut gids = process.user().additional_gids().clone().unwrap_or_default();
            if !gids.contains(&gid) {
                gids.push(gid);
            }
            process.user_mut().set_additional_gids(Some(gids));
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

#[cfg(test)]
mod tests {
    use super::*;
    use oci_spec::runtime::{LinuxNetDevice, Spec};

    fn gen() -> Generator {
        Generator::spec_gen(Some(Spec::default()))
    }

    #[test]
    fn add_device_inserts_replaces_and_remove_deletes() {
        let mut g = gen();
        let mut dev = LinuxDevice::default();
        dev.set_path(PathBuf::from("/dev/x"));
        g.add_device(dev.clone());

        // same path replaces instead of duplicating
        let mut replacement = LinuxDevice::default();
        replacement.set_path(PathBuf::from("/dev/x"));
        replacement.set_major(7);
        g.add_device(replacement);

        let devices = |g: &Generator| {
            g.config
                .as_ref()
                .unwrap()
                .linux()
                .as_ref()
                .unwrap()
                .devices()
                .clone()
                .unwrap_or_default()
        };
        assert_eq!(devices(&g).len(), 1);
        assert_eq!(devices(&g)[0].major(), 7);

        g.remove_device("/dev/x");
        assert!(devices(&g).is_empty());
        // removing an absent device is a no-op
        g.remove_device("/dev/x");
    }

    #[test]
    fn add_linux_resources_device_records_cgroup_entry() {
        let mut g = gen();
        g.add_linux_resources_device(
            true,
            LinuxDeviceType::C,
            Some(1),
            Some(3),
            Some("rwm".to_string()),
        );

        let binding = g.config.unwrap();
        let devices = binding
            .linux()
            .as_ref()
            .unwrap()
            .resources()
            .as_ref()
            .unwrap()
            .devices()
            .as_ref()
            .unwrap();
        assert_eq!(devices.len(), 1);
        assert!(devices[0].allow());
        assert_eq!(devices[0].typ(), Some(LinuxDeviceType::C));
        assert_eq!(devices[0].major(), Some(1));
        assert_eq!(devices[0].minor(), Some(3));
        assert_eq!(devices[0].access().as_deref(), Some("rwm"));
    }

    #[test]
    fn intel_rdt_set_and_clos_id_update() {
        let mut g = gen();
        let mut rdt = LinuxIntelRdt::default();
        rdt.set_clos_id(Some("initial".to_string()));
        g.set_linux_intel_rdt(rdt);
        g.set_linux_intel_rdt_clos_id("updated".to_string());

        let binding = g.config.unwrap();
        let rdt = binding
            .linux()
            .as_ref()
            .unwrap()
            .intel_rdt()
            .as_ref()
            .unwrap();
        assert_eq!(rdt.clos_id().as_deref(), Some("updated"));
    }

    #[test]
    fn additional_gids_deduplicate() {
        let mut g = gen();
        g.add_process_additional_gid(1000);
        g.add_process_additional_gid(1000);
        g.add_process_additional_gid(2000);

        let binding = g.config.unwrap();
        let gids = binding
            .process()
            .as_ref()
            .unwrap()
            .user()
            .additional_gids()
            .clone()
            .unwrap();
        assert_eq!(gids, vec![1000, 2000]);
    }

    #[test]
    fn env_updates_existing_keys_and_appends_new_ones() {
        let mut g = gen();
        g.add_multiple_process_env(&["A=1".to_string(), "B=2".to_string()]);
        g.add_multiple_process_env(&["A=3".to_string(), "C=4".to_string()]);

        let binding = g.config.unwrap();
        let env = binding.process().as_ref().unwrap().env().clone().unwrap();
        // Spec::default() seeds PATH/TERM; assert semantics, not the seed.
        assert_eq!(env.iter().filter(|e| e.starts_with("A=")).count(), 1);
        let tail = &env[env.len() - 3..];
        assert_eq!(tail, ["A=3", "B=2", "C=4"]);
    }

    #[test]
    fn hooks_initialize_then_append() {
        let mut g = gen();
        for _ in 0..2 {
            g.add_prestart_hook(Hook::default());
            g.add_poststart_hook(Hook::default());
            g.add_poststop_hook(Hook::default());
            g.add_createruntime_hook(Hook::default());
            g.add_createcontainer_hook(Hook::default());
            g.add_startcontainer_hook(Hook::default());
        }

        let binding = g.config.unwrap();
        let hooks = binding.hooks().as_ref().unwrap();
        assert_eq!(hooks.prestart().as_ref().unwrap().len(), 2);
        assert_eq!(hooks.poststart().as_ref().unwrap().len(), 2);
        assert_eq!(hooks.poststop().as_ref().unwrap().len(), 2);
        assert_eq!(hooks.create_runtime().as_ref().unwrap().len(), 2);
        assert_eq!(hooks.create_container().as_ref().unwrap().len(), 2);
        assert_eq!(hooks.start_container().as_ref().unwrap().len(), 2);
    }

    #[test]
    fn mounts_add_sort_list_remove_clear() {
        let mut g = gen();
        let mount = |dest: &str| {
            let mut m = Mount::default();
            m.set_destination(PathBuf::from(dest));
            m
        };
        g.add_mount(mount("/z"));
        g.add_mount(mount("/a"));

        g.sort_mounts();
        let dests: Vec<_> = g
            .list_mounts()
            .unwrap()
            .iter()
            .map(|m| m.destination().clone())
            .collect();
        assert!(dests.windows(2).all(|w| w[0] <= w[1]));

        g.remove_mount("/z");
        assert!(!g
            .list_mounts()
            .unwrap()
            .iter()
            .any(|m| m.destination() == &PathBuf::from("/z")));

        g.clear_mounts();
        assert!(g.list_mounts().is_none());
    }

    #[test]
    fn add_process_additional_gid_initializes_empty_gid_list() {
        let mut generator = Generator::spec_gen(Some(oci_spec::runtime::Spec::default()));

        generator.add_process_additional_gid(1000);

        let gids = generator
            .config
            .as_ref()
            .unwrap()
            .process()
            .as_ref()
            .unwrap()
            .user()
            .additional_gids()
            .as_ref()
            .unwrap();
        assert_eq!(gids, &vec![1000]);
    }

    #[test]
    fn add_linux_net_device_initializes_map_and_sets_entry() {
        let mut generator = Generator::spec_gen(Some(oci_spec::runtime::Spec::default()));
        let mut net_device = LinuxNetDevice::default();
        net_device.set_name(Some("container_eth0".to_string()));

        generator.add_linux_net_device("eth0".to_string(), net_device);

        let net_devices = generator
            .config
            .as_ref()
            .unwrap()
            .linux()
            .as_ref()
            .unwrap()
            .net_devices()
            .as_ref()
            .unwrap();
        assert_eq!(
            net_devices.get("eth0").unwrap().name().as_ref().unwrap(),
            "container_eth0"
        );
    }
}
