use crate::cache;
use crate::device;
use crate::spec;
use anyhow::{Error, Result};
use once_cell::sync::OnceCell;

use oci_spec::runtime as oci;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Registry keeps a cache of all CDI Specs installed or generated on
// the host. Registry is the primary interface clients should use to
// interact with CDI.
//
// The most commonly used Registry functions are for refreshing the
// registry and injecting CDI devices into an OCI Spec.
//
pub trait RegistryCache:
    RegistryResolver + RegistryRefresher + RegistryDeviceDB + RegistrySpecDB
{
    fn device_db(&self) -> &Registry;
    fn spec_db(&self) -> &Registry;
}

impl RegistryCache for Registry {
    fn device_db(&self) -> &Registry {
        self
    }
    fn spec_db(&self) -> &Registry {
        self
    }
}

// RegistryRefresher is the registry interface for refreshing the
// cache of CDI Specs and devices.
//
// Configure reconfigures the registry with the given options.
//
// Refresh rescans all CDI Spec directories and updates the
// state of the cache to reflect any changes. It returns any
// errors encountered during the refresh.
//
// GetErrors returns all errors encountered for any of the scanned
// Spec files during the last cache refresh.
//
// GetSpecDirectories returns the set up CDI Spec directories
// currently in use. The directories are returned in the scan
// order of Refresh().
//
// GetSpecDirErrors returns any errors related to the configured
// Spec directories.
pub trait RegistryRefresher {
    //fn configure(&mut self, options: Vec<dyn CacheOption>) -> Result<(), Error>;
    //fn configure<T>(&mut self, option: T) where T: IntoIterator<Item = impl CacheOption>;
    fn configure(&self, options: Vec<Box<dyn cache::CacheOption>>);
    fn refresh(&mut self) -> Result<(), Error>;
    fn get_errors(&self) -> HashMap<String, Vec<Error>>;
    fn get_spec_directories(&self) -> Vec<String>;
    fn get_spec_dir_errors(&self) -> HashMap<String, Error>;
}

impl RegistryRefresher for Registry {
    fn configure(&self, options: Vec<Box<dyn cache::CacheOption>>) {
        self.cache.lock().unwrap().configure(options);
    }
    fn refresh(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn get_errors(&self) -> HashMap<String, Vec<Error>> {
        HashMap::new()
    }
    fn get_spec_directories(&self) -> Vec<String> {
        vec![]
    }
    fn get_spec_dir_errors(&self) -> HashMap<String, Error> {
        HashMap::new()
    }
}

// RegistryResolver is the registry interface for injecting CDI
// devices into an OCI Spec.
//
// InjectDevices takes an OCI Spec and injects into it a set of
// CDI devices given by qualified name. It returns the names of
// any unresolved devices and an error if injection fails.
pub trait RegistryResolver {
    fn inject_devices(
        &self,
        spec: &oci::Spec,
        device: Vec<String>,
    ) -> (Vec<String>, Result<(), Error>);
}

impl RegistryResolver for Registry {
    fn inject_devices(
        &self,
        _spec: &oci::Spec,
        _device: Vec<String>,
    ) -> (Vec<String>, Result<(), Error>) {
        (vec![], Ok(()))
    }
}

// RegistryDeviceDB is the registry interface for querying devices.
//
// GetDevice returns the CDI device for the given qualified name. If
// the device is not GetDevice returns nil.
//
// ListDevices returns a slice with the names of qualified device
// known/. The returned slice is sorted.
pub trait RegistryDeviceDB {
    fn get_device(&self, device: &str) -> device::Device;
    fn list_devices(&self) -> Vec<String>;
}

impl RegistryDeviceDB for Registry {
    fn get_device(&self, _device: &str) -> device::Device {
        device::Device::new()
    }
    fn list_devices(&self) -> Vec<String> {
        vec![]
    }
}

// RegistrySpecDB is the registry interface for querying CDI Specs.
//
// ListVendors returns a slice with all vendors known. The returned
// slice is sorted.
//
// ListClasses returns a slice with all classes known. The returned
// slice is sorted.
//
// GetVendorSpecs returns a slice of all Specs for the vendor.
//
// GetSpecErrors returns any errors for the Spec encountered during
// the last cache refresh.
//
// WriteSpec writes the Spec with the given content and name to the
// last Spec directory.
pub trait RegistrySpecDB {
    fn list_vendors(&self) -> Vec<String>;
    fn list_classes(&self) -> Vec<String>;
    fn get_vendor_specs(&self, vendor: &str) -> Vec<spec::Spec>;
    fn get_spec_errors(&self, spec: &spec::Spec) -> Vec<Error>;
    fn write_spec(&self, raw: &spec::Spec, name: &str) -> Result<(), Error>;
}

impl RegistrySpecDB for Registry {
    fn list_vendors(&self) -> Vec<String> {
        self.cache.lock().unwrap().list_vendors()
    }
    fn list_classes(&self) -> Vec<String> {
        vec![]
    }
    fn get_vendor_specs(&self, vendor: &str) -> Vec<spec::Spec> {
        self.cache.lock().unwrap().get_vendor_specs(vendor)
    }
    fn get_spec_errors(&self, _spec: &spec::Spec) -> Vec<Error> {
        vec![]
    }
    fn write_spec(&self, _raw: &spec::Spec, _name: &str) -> Result<(), Error> {
        Ok(())
    }
}

pub struct Registry {
    pub cache: Arc<Mutex<cache::Cache>>,
}

pub fn get_registry(options: Vec<Box<dyn cache::CacheOption>>) -> Option<Registry> {
    let mut registry: OnceCell<Registry> = OnceCell::new();
    registry.get_or_init(|| Registry {
        cache: cache::Cache::new(),
    });
    registry.get_mut().unwrap().configure(options);
    let _ = registry.get_mut().unwrap().refresh();

    registry.take()
}
