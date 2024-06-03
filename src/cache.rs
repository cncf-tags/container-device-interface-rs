use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use oci_spec::runtime as oci;

use crate::{
    //watch::Watch,
    container_edits::ContainerEdits,
    device::Device,
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

// CacheOption is an option to change some aspect of default CDI behavior.
pub trait CacheOption {
    fn apply(&self, cache: &mut Cache);
}

// WithAutoRefresh returns an option to control automatic Cache refresh.
// By default auto-refresh is enabled, the list of Spec directories are
// monitored and the Cache is automatically refreshed whenever a change
// is detected. This option can be used to disable this behavior when a
// manually refreshed mode is preferable.
pub struct WithAutoRefresh(pub bool);

impl CacheOption for WithAutoRefresh {
    fn apply(&self, cache: &mut Cache) {
        cache.auto_refresh = self.0;
    }
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

pub fn new_cache(options: Vec<Arc<dyn CacheOption>>) -> Arc<Mutex<Cache>> {
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

    pub fn configure(&mut self, options: Vec<Arc<dyn CacheOption>>) {
        for option in options {
            option.apply(self);
        }
    }

    pub fn get_device(&mut self, dev_name: &str) -> Option<&Device> {
        let _ = self.refresh_if_required(false);

        self.devices.get(dev_name)
    }

    pub fn get_devices(&mut self) -> Vec<String> {
        let mut devices: Vec<String> = Vec::new();

        let _ = self.refresh_if_required(false);

        for device in self.specs.keys() {
            devices.push(device.clone());
        }
        devices.sort();
        devices
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
        let specs: HashMap<String, Vec<Spec>> = HashMap::new();
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
            self.specs
                .entry(vendor.clone())
                .or_default()
                .push(s.clone());
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

    pub fn inject_devices(
        &mut self,
        oci_spec: Option<&mut oci::Spec>,
        devices: Vec<String>,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
        let mut unresolved = Vec::new();

        let oci_spec = match oci_spec {
            Some(spec) => spec,
            None => return Err("can't inject devices, OCI Spec is empty".into()),
        };

        let _ = self.refresh_if_required(false);

        let mut edits = ContainerEdits::new();
        let mut specs = HashSet::new();

        for device in devices {
            if let Some(dev) = self.devices.get(&device) {
                let mut spec = dev.get_spec();
                if specs.insert(spec.clone()) {
                    match spec.edits() {
                        Some(ce) => edits.append(ce),
                        None => continue,
                    };
                }
                edits.append(dev.edits());
            } else {
                unresolved.push(device);
            }
        }

        if !unresolved.is_empty() {
            return Err(format!("unresolvable CDI devices {}", unresolved.join(", ")).into());
        }

        if let Err(err) = edits.apply(oci_spec) {
            return Err(format!("failed to inject devices: {}", err).into());
        }

        Ok(Vec::new())
    }

    pub fn get_errors(&self) -> HashMap<String, Vec<anyhow::Error>> {
        // Return errors if any
        HashMap::new()
    }
}
