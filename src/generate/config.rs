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

        if let Some(spec) = self.config.as_mut() {
            if spec.process().is_none() {
                spec.set_process(Some(Process::default()));
            }
        }
    }

    pub fn init_config_linux(&mut self) {
        self.init_config();

        if let Some(spec) = self.config.as_mut() {
            if spec.linux().is_none() {
                spec.set_linux(Some(Linux::default()));
            }
        }
    }

    pub fn init_config_linux_resources(&mut self) {
        self.init_config_linux();

        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if linux.resources().is_none() {
                linux.set_resources(Some(LinuxResources::default()));
            }
        }
    }

    pub fn init_config_hooks(&mut self) {
        self.init_config();

        if let Some(spec) = self.config.as_mut() {
            if spec.hooks().is_none() {
                spec.set_hooks(Some(Hooks::default()));
            }
        }
    }

    pub fn init_config_mounts(&mut self) {
        self.init_config();

        if let Some(spec) = self.config.as_mut() {
            if spec.mounts().is_none() {
                spec.set_mounts(Some(vec![Mount::default()]));
            }
        }
    }

    pub fn init_config_linux_intel_rdt(&mut self) {
        self.init_config_linux();

        if let Some(linux) = self.config.as_mut().unwrap().linux_mut() {
            if linux.intel_rdt().is_none() {
                linux.set_intel_rdt(Some(LinuxIntelRdt::default()));
            }
        }
    }
}
