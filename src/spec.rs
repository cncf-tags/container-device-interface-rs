use std::collections::BTreeMap;

use crate::{container_edits::ContainerEdits, device::Device, specs::config::Spec as CDISpec};

// Spec represents a single CDI Spec. It is usually loaded from a
// file and stored in a cache. The Spec has an associated priority.
// This priority is inherited from the associated priority of the
// CDI Spec directory that contains the CDI Spec file and is used
// to resolve conflicts if multiple CDI Spec files contain entries
// for the same fully qualified device.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Spec {
    cdi_spec: CDISpec,
    vendor: String,
    class: String,
    path: String,
    priority: i32,
    pub devices: BTreeMap<String, Device>,
}

impl Spec {
    // get_vendor returns the vendor of this Spec.
    pub fn get_vendor(&self) -> String {
        self.vendor.clone()
    }

    // get_class returns the device class of this Spec.
    pub fn get_class(&self) -> String {
        self.class.clone()
    }

    // get_devices returns the devices list.
    pub fn get_devices(&self) -> BTreeMap<String, Device> {
        self.devices.clone()
    }

    // get_device returns the device for the given unqualified name.
    pub fn get_device(&self, key: &str) -> Option<&Device> {
        self.devices.get(key)
    }

    // get_path returns the filesystem path of this Spec.
    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    // get_priority returns the priority of this Spec.
    pub fn get_priority(&self) -> i32 {
        self.priority
    }

    // edits returns the applicable global container edits for this spec.
    pub fn edits(&mut self) -> Option<ContainerEdits> {
        self.cdi_spec
            .container_edits
            .clone()
            .map(|ce| ContainerEdits {
                container_edits: ce,
            })
    }
}
