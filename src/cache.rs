use anyhow::Result;
use std::collections::HashMap;

use crate::device::Device;
use crate::spec::Spec;
//use crate::watch::Watch;
use std::sync::{Arc, Mutex};

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
pub struct Cache {
    pub spec_dirs: Vec<String>,
    specs: HashMap<String, Vec<Spec>>,
    devices: HashMap<String, Device>,
    errors: HashMap<String, Vec<Box<dyn std::error::Error + Send + Sync + 'static>>>,
    dir_errors: HashMap<String, Box<dyn std::error::Error + Send + Sync + 'static>>,

    auto_refresh: bool,
    //watch: Watch,
}

impl Cache {
    pub fn new() -> Arc<Mutex<Cache>> {
        Arc::new(Mutex::new(Cache {
            spec_dirs: Vec::new(),
            specs: HashMap::new(),
            devices: HashMap::new(),
            errors: HashMap::new(),
            dir_errors: HashMap::new(),
            auto_refresh: false,
            //watch: Watch::new(),
        }))
    }

    pub fn configure(&mut self, options: Vec<Box<dyn CacheOption>>) {
        for option in options {
            option.apply(self);
        }
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

    fn refresh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
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
}
