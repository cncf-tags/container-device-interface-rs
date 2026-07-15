use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use oci_spec::runtime as oci;

use crate::{
    //watch::Watch,
    container_edits::ContainerEdits,
    device::Device,
    resolved_edits::{CdiEditScope, ResolvedCdiEdits, ScopedContainerEdits},
    spec::Spec,
    spec_dirs::{convert_errors, scan_spec_dirs, with_spec_dirs, SpecError, DEFAULT_SPEC_DIRS},
};

// Define custom errors if not already defined
#[derive(Debug)]
struct ConflictError {
    name: String,
    dev_path: String,
    old_path: String,
}

impl ConflictError {
    fn new(name: &str, dev_path: &str, old_path: &str) -> Self {
        Self {
            name: name.to_owned(),
            dev_path: dev_path.to_owned(),
            old_path: old_path.to_owned(),
        }
    }
}

impl fmt::Display for ConflictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "conflicting device {} (specs {}, {})",
            self.name, self.dev_path, self.old_path
        )
    }
}

impl Error for ConflictError {}

// CdiOption is an option to change some aspect of default CDI behavior.
// We define the CdiOption type using a type alias, which is a Box<dyn FnOnce(&mut Cache)>.
// This means that CdiOption is a trait object that represents a one-time closure that takes a &mut Cache parameter.
pub type CdiOption = Box<dyn FnOnce(&mut Cache)>;

// with_auto_refresh returns an option to control automatic Cache refresh.
// By default auto-refresh is enabled, the list of Spec directories are
// monitored and the Cache is automatically refreshed whenever a change
// is detected. This option can be used to disable this behavior when a
// manually refreshed mode is preferable.
pub fn with_auto_refresh(auto_refresh: bool) -> CdiOption {
    Box::new(move |c: &mut Cache| {
        c.auto_refresh = auto_refresh;
    })
}

#[allow(dead_code)]
#[derive(Default)]
pub struct Cache {
    pub spec_dirs: Vec<String>,
    pub specs: HashMap<String, Vec<Spec>>,
    pub devices: HashMap<String, Device>,
    pub errors: HashMap<String, Vec<Box<dyn std::error::Error + Send + Sync + 'static>>>,
    pub dir_errors: HashMap<String, Box<dyn std::error::Error + Send + Sync + 'static>>,

    pub auto_refresh: bool,
    //watch: Watch,
}

pub fn new_cache(options: Vec<CdiOption>) -> Arc<Mutex<Cache>> {
    let cache = Arc::new(Mutex::new(Cache::default()));

    {
        let mut c = cache.lock().unwrap();

        with_spec_dirs(&DEFAULT_SPEC_DIRS)(&mut c);
        c.configure(options);
        let _ = c.refresh();
    } // MutexGuard `c` is dropped here

    cache
}

impl Cache {
    pub fn new(
        spec_dirs: Vec<String>,
        specs: HashMap<String, Vec<Spec>>,
        devices: HashMap<String, Device>,
    ) -> Self {
        Self {
            spec_dirs,
            specs,
            devices,
            errors: HashMap::new(),
            dir_errors: HashMap::new(),
            auto_refresh: false,
            //watch: Watch::new(),
        }
    }

    pub fn configure(&mut self, options: Vec<CdiOption>) {
        for option in options {
            option(self);
        }
    }

    pub fn get_device(&mut self, dev_name: &str) -> Option<&Device> {
        let _ = self.refresh_if_required(false);

        self.devices.get(dev_name)
    }

    pub fn list_devices(&mut self) -> Vec<String> {
        let _ = self.refresh_if_required(false);

        let mut devices: Vec<String> = self.devices.keys().cloned().collect();
        devices.sort();
        devices
    }

    pub fn list_vendors(&mut self) -> Vec<String> {
        let mut vendors: Vec<String> = Vec::new();

        let _ = self.refresh_if_required(false);

        for vendor in self.specs.keys() {
            vendors.push(vendor.clone());
        }
        vendors.sort();
        vendors
    }

    pub fn get_vendor_specs(&mut self, vendor: &str) -> Vec<Spec> {
        let _ = self.refresh_if_required(false);

        match self.specs.get(vendor) {
            Some(specs) => specs.clone(),
            None => Vec::new(),
        }
    }

    // refresh the Cache by rescanning CDI Spec directories and files.
    pub fn refresh(&mut self) -> Result<(), Box<dyn Error>> {
        let mut specs: HashMap<String, Vec<Spec>> = HashMap::new();
        let mut devices: HashMap<String, Device> = HashMap::new();
        let mut conflicts: HashSet<String> = HashSet::new();
        let mut spec_errors: HashMap<String, Vec<Box<dyn Error>>> = HashMap::new();

        // Wrap collect_error and resolve_conflict in RefCell
        let collect_error = RefCell::new(|err: Box<dyn Error>, paths: Vec<String>| {
            let err_string = err.to_string();
            for path in paths {
                spec_errors
                    .entry(path.to_string())
                    .or_default()
                    .push(Box::new(SpecError::new(&err_string.to_string())));
            }
        });

        let resolve_conflict = RefCell::new(|name: &str, dev: &Device, old: &Device| -> bool {
            let dev_spec = dev.get_spec();
            let old_spec = old.get_spec();
            let dev_prio = dev_spec.get_priority();
            let old_prio = old_spec.get_priority();

            match dev_prio.cmp(&old_prio) {
                std::cmp::Ordering::Greater => false,
                std::cmp::Ordering::Equal => {
                    let dev_path = dev_spec.get_path();
                    let old_path = old_spec.get_path();
                    collect_error.borrow_mut()(
                        Box::new(ConflictError::new(name, &dev_path, &old_path)),
                        vec![dev_path.clone(), old_path.clone()],
                    );
                    conflicts.insert(name.to_owned());
                    true
                }
                std::cmp::Ordering::Less => true,
            }
        });

        let mut scan_spec_fn = |s: Spec| -> Result<(), Box<dyn Error>> {
            let vendor = s.get_vendor().to_owned();
            specs.entry(vendor.clone()).or_default().push(s.clone());
            let spec_devices = s.get_devices();
            for dev in spec_devices.values() {
                let qualified = dev.get_qualified_name();
                if let Some(other) = devices.get(&qualified) {
                    if resolve_conflict.borrow_mut()(&qualified, dev, other) {
                        continue;
                    }
                }
                devices.insert(qualified, dev.clone());
            }

            Ok(())
        };

        let scaned_specs: Vec<Spec> = scan_spec_dirs(&self.spec_dirs)?;
        for spec in scaned_specs {
            scan_spec_fn(spec)?
        }

        for conflict in conflicts.iter() {
            self.devices.remove(conflict);
        }

        self.specs = specs;
        self.devices = devices;
        self.errors = convert_errors(&spec_errors);

        let errs: Vec<String> = spec_errors
            .values()
            .flat_map(|errors| errors.iter().map(|err| err.to_string()))
            .collect();

        if !errs.is_empty() {
            Err(errs.join(", ").into())
        } else {
            Ok(())
        }
    }

    fn refresh_if_required(&mut self, force: bool) -> Result<bool, Box<dyn std::error::Error>> {
        // We need to refresh if
        // - it's forced by an explicit call to Refresh() in manual mode
        // - a missing Spec dir appears (added to watch) in auto-refresh mode
        // TODO: Here it will be recoverd if watch is completed.
        // if force || (self.auto_refresh && self.watch.update(&mut self.dir_errors, vec![])) {
        if force || (self.auto_refresh) {
            self.refresh()?;
            return Ok(true);
        }

        Ok(false)
    }

    fn collect_scoped_container_edits(
        &mut self,
        devices: &[String],
        propagate_refresh_errors: bool,
    ) -> Result<Vec<ScopedContainerEdits>, Box<dyn Error + Send + Sync + 'static>> {
        let mut unresolved = Vec::new();

        let refresh_result = self.refresh_if_required(false);
        if propagate_refresh_errors {
            refresh_result.map_err(|err| -> Box<dyn Error + Send + Sync + 'static> {
                err.to_string().into()
            })?;
        }

        let mut scoped_edits = Vec::new();
        let mut specs: HashSet<Spec> = HashSet::new();

        for device in devices {
            if let Some(dev) = self.devices.get(device) {
                let mut spec = dev.get_spec();
                let spec_kind = spec.cdi_spec.kind.clone();
                let spec_path = PathBuf::from(spec.get_path());

                if specs.insert(spec.clone()) {
                    if let Some(edits) = spec.edits() {
                        scoped_edits.push(ScopedContainerEdits {
                            edits,
                            scope: CdiEditScope::Spec {
                                kind: spec_kind.clone(),
                                path: spec_path.clone(),
                            },
                        });
                    }
                }

                scoped_edits.push(ScopedContainerEdits {
                    edits: dev.edits(),
                    scope: CdiEditScope::Device {
                        qualified_name: device.clone(),
                        spec_kind,
                        spec_path,
                    },
                });
            } else {
                unresolved.push(device.clone());
            }
        }

        if !unresolved.is_empty() {
            return Err(format!("unresolvable CDI devices {}", unresolved.join(", ")).into());
        }

        Ok(scoped_edits)
    }

    pub fn inject_devices(
        &mut self,
        oci_spec: Option<&mut oci::Spec>,
        devices: Vec<String>,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
        let oci_spec = match oci_spec {
            Some(spec) => spec,
            None => return Err("can't inject devices, OCI Spec is empty".into()),
        };

        let scoped_edits = self.collect_scoped_container_edits(&devices, false)?;
        let edits = &mut ContainerEdits::new();

        for scoped in scoped_edits {
            edits.append(scoped.edits)?;
        }

        if let Err(err) = edits.apply(oci_spec) {
            return Err(format!("failed to inject devices: {}", err).into());
        }

        Ok(Vec::new())
    }

    pub fn resolve_edits(&mut self, devices: &[String]) -> Result<ResolvedCdiEdits> {
        let scoped_edits = self
            .collect_scoped_container_edits(devices, true)
            .map_err(anyhow::Error::from_boxed)?;
        let mut resolved = ResolvedCdiEdits::default();

        for scoped in scoped_edits {
            resolved.append_container_edits(&scoped.edits, scoped.scope)?;
        }

        Ok(resolved)
    }

    pub fn get_errors(&self) -> HashMap<String, Vec<anyhow::Error>> {
        // Return errors if any
        HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_dirs::with_spec_dirs;
    use crate::{
        spec::{new_spec, Spec as LoadedSpec},
        specs::config::{
            ContainerEdits as CDIContainerEdits, Device as CDIDevice, DeviceNode, Hook, IntelRdt,
            LinuxNetDevice, Mount, Spec as CDISpec,
        },
        CdiEditScope, ResolvedCdiDeviceNode, ResolvedCdiMount, UnsupportedCdiEdit,
        UnsupportedCdiEditKind,
    };
    use oci_spec::runtime::Spec as OCISpec;
    use std::{collections::HashMap, fs, path::PathBuf};

    fn spec_yaml(kind: &str, env: &str) -> String {
        format!(
            r#"cdiVersion: "0.6.0"
kind: "{kind}"
devices:
  - name: "gpu0"
    containerEdits:
      env:
        - "{env}"
"#
        )
    }

    fn dir_cache(dirs: &[&str]) -> Cache {
        let mut cache = Cache::default();
        with_spec_dirs(dirs)(&mut cache);
        cache
    }

    #[test]
    fn refresh_scans_dirs_and_answers_queries() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("vendor.yaml"),
            spec_yaml("vendor.com/device", "VENDOR=1"),
        )
        .unwrap();
        let mut cache = dir_cache(&[dir.path().to_str().unwrap()]);

        cache.refresh().unwrap();

        assert_eq!(cache.list_devices(), vec!["vendor.com/device=gpu0"]);
        assert_eq!(cache.list_vendors(), vec!["vendor.com"]);
        assert_eq!(cache.get_vendor_specs("vendor.com").len(), 1);
        assert!(cache.get_vendor_specs("other.com").is_empty());
        assert!(cache.get_device("vendor.com/device=gpu0").is_some());
        assert!(cache.get_device("vendor.com/device=missing").is_none());
    }

    #[test]
    fn auto_refresh_picks_up_new_specs_without_manual_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = dir_cache(&[dir.path().to_str().unwrap()]);
        with_auto_refresh(true)(&mut cache);
        assert!(cache.list_devices().is_empty());

        fs::write(
            dir.path().join("vendor.yaml"),
            spec_yaml("vendor.com/device", "VENDOR=1"),
        )
        .unwrap();

        // No explicit refresh(): the query must trigger it.
        assert_eq!(cache.list_devices(), vec!["vendor.com/device=gpu0"]);
    }

    #[test]
    fn later_dir_wins_on_conflicting_device_names() {
        let low = tempfile::tempdir().unwrap();
        let high = tempfile::tempdir().unwrap();
        fs::write(
            low.path().join("a.yaml"),
            spec_yaml("vendor.com/device", "FROM=low"),
        )
        .unwrap();
        fs::write(
            high.path().join("b.yaml"),
            spec_yaml("vendor.com/device", "FROM=high"),
        )
        .unwrap();
        let mut cache = dir_cache(&[low.path().to_str().unwrap(), high.path().to_str().unwrap()]);

        cache.refresh().unwrap();

        let dev = cache.get_device("vendor.com/device=gpu0").unwrap();
        assert_eq!(dev.get_spec().get_priority(), 1);
    }

    #[test]
    fn same_priority_conflicts_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.yaml"),
            spec_yaml("vendor.com/device", "FROM=a"),
        )
        .unwrap();
        fs::write(
            dir.path().join("b.yaml"),
            spec_yaml("vendor.com/device", "FROM=b"),
        )
        .unwrap();
        let mut cache = dir_cache(&[dir.path().to_str().unwrap()]);

        let err = cache.refresh().unwrap_err();

        assert!(err.to_string().contains("conflicting device"));
        assert!(!cache.errors.is_empty());
    }

    #[test]
    fn inject_devices_requires_an_oci_spec() {
        let mut cache = Cache::default();
        let err = cache.inject_devices(None, vec![]).unwrap_err();
        assert!(err.to_string().contains("OCI Spec is empty"));
    }

    #[test]
    fn inject_devices_reports_unresolvable_devices() {
        let mut cache = Cache::default();
        let mut oci_spec = OCISpec::default();
        let err = cache
            .inject_devices(Some(&mut oci_spec), vec!["vendor.com/device=nope".into()])
            .unwrap_err();
        assert!(err.to_string().contains("unresolvable CDI devices"));
        assert!(err.to_string().contains("vendor.com/device=nope"));
    }

    fn spec_path(index: usize) -> PathBuf {
        PathBuf::from(format!("/tmp/cdi-resolved-edits-{index}.yaml"))
    }

    fn raw_spec(
        kind: &str,
        container_edits: Option<CDIContainerEdits>,
        devices: Vec<CDIDevice>,
    ) -> CDISpec {
        CDISpec {
            version: "1.1.0".to_string(),
            kind: kind.to_string(),
            container_edits,
            devices,
            ..Default::default()
        }
    }

    fn raw_device(name: &str, container_edits: CDIContainerEdits) -> CDIDevice {
        CDIDevice {
            name: name.to_string(),
            container_edits,
            ..Default::default()
        }
    }

    fn env_edits(values: &[&str]) -> CDIContainerEdits {
        CDIContainerEdits {
            env: Some(values.iter().map(|value| value.to_string()).collect()),
            ..Default::default()
        }
    }

    fn node_edits(
        path: &str,
        host_path: Option<&str>,
        typ: Option<&str>,
        major: Option<i64>,
        minor: Option<i64>,
    ) -> CDIContainerEdits {
        CDIContainerEdits {
            device_nodes: Some(vec![DeviceNode {
                path: path.to_string(),
                host_path: host_path.map(str::to_string),
                r#type: typ.map(str::to_string),
                major,
                minor,
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn mount_edits() -> CDIContainerEdits {
        CDIContainerEdits {
            mounts: Some(vec![Mount {
                host_path: "/host/data".to_string(),
                container_path: "/container/data".to_string(),
                r#type: Some("bind".to_string()),
                options: Some(vec!["ro".to_string(), "rbind".to_string()]),
            }]),
            ..Default::default()
        }
    }

    fn unsupported_edits() -> CDIContainerEdits {
        CDIContainerEdits {
            hooks: Some(vec![Hook {
                hook_name: "prestart".to_string(),
                path: "/bin/true".to_string(),
                ..Default::default()
            }]),
            net_devices: Some(vec![LinuxNetDevice {
                host_interface_name: "eth-test0".to_string(),
                name: "eth0".to_string(),
            }]),
            intel_rdt: Some(IntelRdt {
                clos_id: Some("class-a".to_string()),
                ..Default::default()
            }),
            additional_gids: Some(vec![44, 45]),
            ..Default::default()
        }
    }

    fn cache_from_raw_specs(raw_specs: Vec<CDISpec>) -> Cache {
        let mut specs: HashMap<String, Vec<LoadedSpec>> = HashMap::new();
        let mut devices = HashMap::new();

        for (index, raw) in raw_specs.iter().enumerate() {
            let spec = new_spec(raw, &spec_path(index), 0).unwrap();
            for device in spec.get_devices().values() {
                devices.insert(device.get_qualified_name(), device.clone());
            }
            specs.entry(spec.get_vendor()).or_default().push(spec);
        }

        Cache::new(Vec::new(), specs, devices)
    }

    fn spec_scope(index: usize) -> CdiEditScope {
        CdiEditScope::Spec {
            kind: "vendor.com/device".to_string(),
            path: spec_path(index),
        }
    }

    fn device_scope(qualified_name: &str, index: usize) -> CdiEditScope {
        CdiEditScope::Device {
            qualified_name: qualified_name.to_string(),
            spec_kind: "vendor.com/device".to_string(),
            spec_path: spec_path(index),
        }
    }

    #[test]
    fn resolve_edits_propagates_refresh_errors() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            temp_dir.path().join("bad.yaml"),
            r#"cdiVersion: "1.1.0"
kind: "vendor.com/device"
unknownTopLevel: true
devices:
  - name: "gpu0"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#,
        )
        .unwrap();
        let mut cache = Cache::new(
            vec![temp_dir.path().display().to_string()],
            HashMap::new(),
            HashMap::new(),
        );
        cache.auto_refresh = true;

        assert!(cache.resolve_edits(&[]).is_err());
    }

    #[test]
    fn inject_devices_ignores_auto_refresh_errors_and_uses_existing_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            temp_dir.path().join("bad.yaml"),
            r#"cdiVersion: "1.1.0"
kind: "vendor.com/device"
unknownTopLevel: true
devices:
  - name: "other"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#,
        )
        .unwrap();

        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits("/dev/null", None, Some("c"), Some(1), Some(3)),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);
        cache.spec_dirs = vec![temp_dir.path().display().to_string()];
        cache.auto_refresh = true;
        let mut oci_spec = OCISpec::default();

        cache
            .inject_devices(
                Some(&mut oci_spec),
                vec!["vendor.com/device=gpu0".to_string()],
            )
            .unwrap();

        let devices = oci_spec
            .linux()
            .as_ref()
            .unwrap()
            .devices()
            .as_ref()
            .unwrap();
        assert_eq!(PathBuf::from("/dev/null"), devices[0].path().clone());
    }

    #[test]
    fn inject_devices_preserves_spec_level_intel_rdt_with_device_edits() {
        let raw = CDISpec {
            version: "1.1.0".to_string(),
            kind: "vendor.com/device".to_string(),
            container_edits: Some(CDIContainerEdits {
                intel_rdt: Some(IntelRdt {
                    clos_id: Some("global-class".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            devices: vec![CDIDevice {
                name: "gpu0".to_string(),
                container_edits: CDIContainerEdits {
                    device_nodes: Some(vec![DeviceNode {
                        path: "/dev/null".to_string(),
                        r#type: Some("c".to_string()),
                        major: Some(1),
                        minor: Some(3),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let spec = new_spec(&raw, &PathBuf::from("/tmp/vendor-device.yaml"), 0).unwrap();
        let device = spec.get_device("gpu0").unwrap().clone();
        let mut devices = HashMap::new();
        devices.insert(device.get_qualified_name(), device);
        let mut cache = Cache::new(Vec::new(), HashMap::new(), devices);
        let mut oci_spec = OCISpec::default();

        cache
            .inject_devices(
                Some(&mut oci_spec),
                vec!["vendor.com/device=gpu0".to_string()],
            )
            .unwrap();

        let intel_rdt = oci_spec
            .linux()
            .as_ref()
            .unwrap()
            .intel_rdt()
            .as_ref()
            .unwrap();
        assert_eq!(
            Some(&"global-class".to_string()),
            intel_rdt.clos_id().as_ref()
        );
    }

    #[test]
    fn resolve_edits_preserves_host_path_when_it_differs_from_container_path() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits(
                    "/dev/codex0",
                    Some("/dev/null"),
                    Some("c"),
                    Some(1),
                    Some(3),
                ),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(
            vec![ResolvedCdiDeviceNode {
                host_path: PathBuf::from("/dev/null"),
                container_path: PathBuf::from("/dev/codex0"),
                typ: Some("c".to_string()),
                major: Some(1),
                minor: Some(3),
                file_mode: None,
                permissions: None,
                uid: None,
                gid: None,
                scope: device_scope("vendor.com/device=gpu0", 0),
            }],
            edits.device_nodes
        );
    }

    #[test]
    fn resolve_edits_defaults_missing_host_path_to_container_path() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits("/dev/null", None, Some("c"), Some(1), Some(3)),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(PathBuf::from("/dev/null"), edits.device_nodes[0].host_path);
        assert_eq!(
            PathBuf::from("/dev/null"),
            edits.device_nodes[0].container_path
        );
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "miri's stat shim does not populate rdev; /dev/null major/minor read as 0"
    )]
    fn resolve_edits_treats_empty_device_type_as_missing() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits("/dev/null", None, Some(""), None, None),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(Some("c"), edits.device_nodes[0].typ.as_deref());
        assert_eq!(Some(1), edits.device_nodes[0].major);
        assert_eq!(Some(3), edits.device_nodes[0].minor);
    }

    #[test]
    fn resolve_edits_preserves_mount_fields() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device("gpu0", mount_edits())],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(
            vec![ResolvedCdiMount {
                host_path: PathBuf::from("/host/data"),
                container_path: PathBuf::from("/container/data"),
                typ: Some("bind".to_string()),
                options: vec!["ro".to_string(), "rbind".to_string()],
                scope: device_scope("vendor.com/device=gpu0", 0),
            }],
            edits.mounts
        );
    }

    #[test]
    fn resolve_edits_applies_spec_level_edits_once_for_two_devices_from_same_spec() {
        let raw = raw_spec(
            "vendor.com/device",
            Some(env_edits(&["SPEC=1"])),
            vec![
                raw_device("gpu0", env_edits(&["DEV=0"])),
                raw_device("gpu1", env_edits(&["DEV=1"])),
            ],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&[
                "vendor.com/device=gpu0".to_string(),
                "vendor.com/device=gpu1".to_string(),
            ])
            .unwrap();

        assert_eq!(
            1,
            edits
                .env
                .iter()
                .filter(|value| value.as_str() == "SPEC=1")
                .count()
        );
    }

    #[test]
    fn resolve_edits_applies_spec_edits_before_first_device_edits() {
        let raw = raw_spec(
            "vendor.com/device",
            Some(env_edits(&["SPEC=1"])),
            vec![
                raw_device("gpu0", env_edits(&["DEV=0"])),
                raw_device("gpu1", env_edits(&["DEV=1"])),
            ],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&[
                "vendor.com/device=gpu0".to_string(),
                "vendor.com/device=gpu1".to_string(),
            ])
            .unwrap();

        assert_eq!(
            vec![
                "SPEC=1".to_string(),
                "DEV=0".to_string(),
                "DEV=1".to_string()
            ],
            edits.env
        );
    }

    #[test]
    fn resolve_edits_reports_spec_level_unsupported_edits() {
        let raw = raw_spec(
            "vendor.com/device",
            Some(unsupported_edits()),
            vec![raw_device("gpu0", env_edits(&["DEV=0"]))],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);
        let scope = spec_scope(0);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(
            vec![
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::Hooks,
                    count: 1,
                    scope: scope.clone(),
                },
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::NetDevices,
                    count: 1,
                    scope: scope.clone(),
                },
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::IntelRdt,
                    count: 1,
                    scope: scope.clone(),
                },
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::AdditionalGids,
                    count: 2,
                    scope,
                },
            ],
            edits.unsupported
        );
    }

    #[test]
    fn resolve_edits_reports_device_level_unsupported_edits() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device("gpu0", unsupported_edits())],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);
        let scope = device_scope("vendor.com/device=gpu0", 0);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(
            vec![
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::Hooks,
                    count: 1,
                    scope: scope.clone(),
                },
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::NetDevices,
                    count: 1,
                    scope: scope.clone(),
                },
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::IntelRdt,
                    count: 1,
                    scope: scope.clone(),
                },
                UnsupportedCdiEdit {
                    kind: UnsupportedCdiEditKind::AdditionalGids,
                    count: 2,
                    scope,
                },
            ],
            edits.unsupported
        );
    }

    #[test]
    fn resolve_edits_errors_when_requested_device_is_absent() {
        let mut cache = Cache::new(Vec::new(), HashMap::new(), HashMap::new());

        let err = cache
            .resolve_edits(&["vendor.com/device=missing".to_string()])
            .unwrap_err();

        assert!(
            err.to_string()
                .contains("unresolvable CDI devices vendor.com/device=missing"),
            "{err:?}"
        );
    }

    #[test]
    fn resolve_edits_errors_when_fully_specified_host_path_is_missing() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits(
                    "/container/missing",
                    Some("/definitely/not/a/cdi/device"),
                    Some("c"),
                    Some(1),
                    Some(3),
                ),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let err = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap_err();
        let err = format!("{err:?}");

        assert!(
            err.contains("failed to inspect CDI device node host path")
                || err.contains("/definitely/not/a/cdi/device"),
            "{err}"
        );
    }

    #[test]
    fn resolve_edits_errors_when_fully_specified_host_path_type_mismatches() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits(
                    "/container/null",
                    Some("/dev/null"),
                    Some("b"),
                    Some(1),
                    Some(3),
                ),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let err = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap_err();
        let err = format!("{err:?}");

        assert!(err.contains("host type mismatch"), "{err}");
    }

    #[test]
    fn resolve_edits_accepts_unbuffered_char_type_for_char_host_path() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits(
                    "/container/null",
                    Some("/dev/null"),
                    Some("u"),
                    Some(1),
                    Some(3),
                ),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(Some("u"), edits.device_nodes[0].typ.as_deref());
        assert_eq!(Some(1), edits.device_nodes[0].major);
        assert_eq!(Some(3), edits.device_nodes[0].minor);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "miri's stat shim does not populate rdev; /dev/null major/minor read as 0"
    )]
    fn resolve_edits_fills_unbuffered_char_metadata_from_char_host_path() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits("/container/null", Some("/dev/null"), Some("u"), None, None),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(Some("u"), edits.device_nodes[0].typ.as_deref());
        assert_eq!(Some(1), edits.device_nodes[0].major);
        assert_eq!(Some(3), edits.device_nodes[0].minor);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "miri's stat shim does not populate rdev; /dev/null major/minor read as 0"
    )]
    fn resolve_edits_fills_missing_device_metadata_from_host_path() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits("/container/null", Some("/dev/null"), None, None, None),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(
            vec![ResolvedCdiDeviceNode {
                host_path: PathBuf::from("/dev/null"),
                container_path: PathBuf::from("/container/null"),
                typ: Some("c".to_string()),
                major: Some(1),
                minor: Some(3),
                file_mode: None,
                permissions: None,
                uid: None,
                gid: None,
                scope: device_scope("vendor.com/device=gpu0", 0),
            }],
            edits.device_nodes
        );
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "miri's stat shim does not populate rdev; /dev/null major/minor read as 0"
    )]
    fn resolve_edits_fills_missing_minor_when_major_is_present() {
        let raw = raw_spec(
            "vendor.com/device",
            None,
            vec![raw_device(
                "gpu0",
                node_edits(
                    "/container/null",
                    Some("/dev/null"),
                    Some("c"),
                    Some(1),
                    None,
                ),
            )],
        );
        let mut cache = cache_from_raw_specs(vec![raw]);

        let edits = cache
            .resolve_edits(&["vendor.com/device=gpu0".to_string()])
            .unwrap();

        assert_eq!(Some(3), edits.device_nodes[0].minor);
    }
}
