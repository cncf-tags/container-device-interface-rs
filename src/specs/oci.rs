use std::path::PathBuf;

use anyhow::Result;
use oci_spec::runtime::{
    Hook as OCIHook, LinuxDevice, LinuxDeviceType, LinuxIntelRdt, Mount as OCIMount,
};

use crate::specs::config::{DeviceNode, Hook as CDIHook, IntelRdt, Mount as CDIMount};

impl CDIHook {
    pub fn to_oci(&self) -> Result<OCIHook> {
        let mut oci_hook: OCIHook = Default::default();
        oci_hook.set_path(PathBuf::from(&self.path));
        oci_hook.set_args(self.args.clone());
        oci_hook.set_env(self.env.clone());
        oci_hook.set_timeout(self.timeout);

        Ok(oci_hook)
    }
}

impl CDIMount {
    pub fn to_oci(&self) -> Result<OCIMount> {
        let mut oci_mount: OCIMount = Default::default();
        oci_mount.set_source(Some(PathBuf::from(&self.host_path)));
        oci_mount.set_destination(PathBuf::from(&self.container_path));
        oci_mount.set_typ(self.r#type.clone());
        oci_mount.set_options(self.options.clone());

        Ok(oci_mount)
    }
}

impl DeviceNode {
    pub fn to_oci(&self) -> Result<LinuxDevice> {
        let mut linux_device: LinuxDevice = Default::default();
        linux_device.set_path(PathBuf::from(&self.path));
        if let Some(_typ) = &self.r#type {
            linux_device.set_typ(LinuxDeviceType::C);
        }
        if let Some(maj) = self.major {
            linux_device.set_major(maj);
        };
        if let Some(min) = self.minor {
            linux_device.set_minor(min);
        }
        linux_device.set_file_mode(self.file_mode);
        linux_device.set_uid(self.uid);
        linux_device.set_gid(self.gid);

        Ok(linux_device)
    }
}

impl IntelRdt {
    pub fn to_oci(&self) -> Result<LinuxIntelRdt> {
        let mut intel_rdt: LinuxIntelRdt = Default::default();
        intel_rdt.set_clos_id(self.clos_id.clone());
        intel_rdt.set_l3_cache_schema(self.l3_cache_schema.clone());
        intel_rdt.set_mem_bw_schema(self.mem_bw_schema.clone());
        intel_rdt.set_enable_cmt(Some(self.enable_cmt));
        intel_rdt.set_enable_mbm(Some(self.enable_mbm));

        Ok(intel_rdt)
    }
}

#[cfg(test)]
mod tests {
    use oci_spec::runtime::LinuxDevice;
    use std::path::PathBuf;

    use crate::specs::{
        config::DeviceNode,
        oci::{CDIHook, CDIMount, OCIHook, OCIMount},
    };

    #[test]
    fn test_hooks_to_oci() {
        let cdi_hooks = CDIHook {
            hook_name: "x".to_owned(),
            path: "y".to_owned(),
            args: None,
            env: Some(vec!["n".to_owned(), "v".to_owned()]),
            timeout: Some(100_i64),
        };

        let oci_hook: OCIHook = cdi_hooks.to_oci().unwrap();
        assert_eq!(PathBuf::from(cdi_hooks.path), oci_hook.path().clone());
        assert_eq!(None, oci_hook.args().clone());
        assert_eq!(cdi_hooks.env, oci_hook.env().clone());
        assert_eq!(cdi_hooks.timeout, oci_hook.timeout());
    }

    #[test]
    fn test_mount_to_oci() {
        let cdi_mount = CDIMount {
            host_path: "x".to_owned(),
            container_path: "c".to_owned(),
            r#type: Some("t".to_owned()),
            options: None,
        };

        let oci_mount: OCIMount = cdi_mount.to_oci().unwrap();
        assert_eq!(
            Some(PathBuf::from(cdi_mount.host_path)),
            oci_mount.source().clone()
        );
        assert_eq!(
            PathBuf::from(cdi_mount.container_path),
            oci_mount.destination().clone()
        );
        assert_eq!(cdi_mount.r#type, oci_mount.typ().clone());
    }

    #[test]
    fn test_device_node_to_oci() {
        let dev_node = DeviceNode {
            path: "p".to_owned(),
            host_path: Some("hostp".to_owned()),
            major: Some(251),
            minor: Some(0),
            ..Default::default()
        };

        let linux_dev: LinuxDevice = dev_node.to_oci().unwrap();
        assert_eq!(PathBuf::from(dev_node.path), linux_dev.path().clone());
        assert_eq!(dev_node.major, Some(linux_dev.major()));
        assert_eq!(dev_node.minor, Some(linux_dev.minor()));
    }
}
