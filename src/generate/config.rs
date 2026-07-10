use std::collections::HashMap;

use oci_spec::runtime::{Hooks, Linux, LinuxIntelRdt, LinuxResources, Mount, Process, Spec};

pub struct Generator {
    pub config: Option<Spec>,
    pub host_specific: bool,
    pub env_map: HashMap<String, usize>,
}

impl Generator {
    pub fn spec_gen(spec: Option<Spec>) -> Self {
        Generator {
            config: spec,
            host_specific: false,
            env_map: HashMap::new(),
        }
    }

    pub fn init_config(&mut self) {
        if self.config.is_none() {
            self.config = Some(Spec::default());
        }
    }

    pub fn init_config_process(&mut self) {
        self.init_config();

        let spec = self.config.as_mut().unwrap();
        if spec.process().is_none() {
            spec.set_process(Some(Process::default()));
        }
    }

    pub fn init_config_linux(&mut self) {
        self.init_config();

        let spec = self.config.as_mut().unwrap();
        if spec.linux().is_none() {
            spec.set_linux(Some(Linux::default()));
        }
    }

    pub fn init_config_linux_resources(&mut self) {
        self.init_config_linux();

        let linux = self.config.as_mut().unwrap().linux_mut().as_mut().unwrap();
        if linux.resources().is_none() {
            linux.set_resources(Some(LinuxResources::default()));
        }
    }

    pub fn init_config_linux_resources_devices(&mut self) {
        self.init_config_linux_resources();

        let linux = self.config.as_mut().unwrap().linux_mut().as_mut().unwrap();
        let resource = linux.resources_mut().as_mut().unwrap();
        if resource.devices().is_none() {
            resource.set_devices(Some(Vec::new()));
        }
    }

    pub fn init_config_linux_net_devices(&mut self) {
        self.init_config_linux();

        let linux = self.config.as_mut().unwrap().linux_mut().as_mut().unwrap();
        if linux.net_devices().is_none() {
            linux.set_net_devices(Some(HashMap::new()));
        }
    }

    pub fn init_config_hooks(&mut self) {
        self.init_config();

        let spec = self.config.as_mut().unwrap();
        if spec.hooks().is_none() {
            spec.set_hooks(Some(Hooks::default()));
        }
    }

    pub fn init_config_mounts(&mut self) {
        self.init_config();

        let spec = self.config.as_mut().unwrap();
        if spec.mounts().is_none() {
            spec.set_mounts(Some(vec![Mount::default()]));
        }
    }

    pub fn init_config_linux_intel_rdt(&mut self) {
        self.init_config_linux();

        let linux = self.config.as_mut().unwrap().linux_mut().as_mut().unwrap();
        if linux.intel_rdt().is_none() {
            linux.set_intel_rdt(Some(LinuxIntelRdt::default()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each init_* must create the layer once and leave it alone after -
    // calling twice exercises both branches.
    #[test]
    fn init_helpers_are_idempotent() {
        // An all-None spec drives the create arms (serde defaults would
        // repopulate, hence explicit setters); the second loop pass and the
        // pre-populated Spec::default() drive the leave-alone arms.
        let mut bare = Spec::default();
        bare.set_process(None)
            .set_linux(None)
            .set_hooks(None)
            .set_mounts(None);
        // Linux::default() pre-populates resources: hollow it out layer by
        // layer so the set_resources / set_devices create arms fire too.
        let mut hollow_linux = Linux::default();
        hollow_linux
            .set_resources(None)
            .set_net_devices(None)
            .set_intel_rdt(None);
        let mut spec_hollow_linux = Spec::default();
        spec_hollow_linux.set_linux(Some(hollow_linux));
        let mut resources_no_devices = LinuxResources::default();
        resources_no_devices.set_devices(None);
        let mut linux_no_devices = Linux::default();
        linux_no_devices.set_resources(Some(resources_no_devices));
        let mut spec_no_devices = Spec::default();
        spec_no_devices.set_linux(Some(linux_no_devices));

        for spec in [
            Some(bare),
            Some(spec_hollow_linux),
            Some(spec_no_devices),
            None,
            Some(Spec::default()),
        ] {
            let mut g = Generator::spec_gen(spec);
            g.init_config();
            assert!(g.config.is_some());

            for _ in 0..2 {
                g.init_config_process();
                g.init_config_linux();
                g.init_config_linux_resources();
                g.init_config_linux_resources_devices();
                g.init_config_linux_net_devices();
                g.init_config_hooks();
                g.init_config_mounts();
                g.init_config_linux_intel_rdt();
            }

            let spec = g.config.as_ref().unwrap();
            assert!(spec.process().is_some());
            assert!(spec.hooks().is_some());
            assert!(spec.mounts().is_some());
            let linux = spec.linux().as_ref().unwrap();
            assert!(linux.resources().as_ref().unwrap().devices().is_some());
            assert!(linux.net_devices().is_some());
            assert!(linux.intel_rdt().is_some());
        }
    }
}
