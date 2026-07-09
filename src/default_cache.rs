use anyhow::Result;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use oci_spec::runtime::Spec;
use once_cell::sync::OnceCell;

use crate::cache::{new_cache, with_auto_refresh, Cache, CdiOption};

// Process-wide: configure() and the query helpers must observe the same
// instance, and auto-refresh state has to survive across calls.
static DEFAULT_CACHE: OnceCell<Arc<Mutex<Cache>>> = OnceCell::new();

fn get_or_create_default_cache() -> Arc<Mutex<Cache>> {
    DEFAULT_CACHE
        .get_or_init(|| new_cache(vec![with_auto_refresh(true)]))
        .clone()
}

pub fn get_default_cache() -> Arc<Mutex<Cache>> {
    get_or_create_default_cache()
}

pub fn configure(options: Vec<CdiOption>) -> Result<()> {
    let cache = get_or_create_default_cache();
    let mut cache = cache.lock().unwrap();
    if options.is_empty() {
        return Ok(());
    }
    cache.configure(options);
    Ok(())
}

pub fn refresh() -> Result<(), Box<dyn Error>> {
    let cache = get_default_cache();
    let mut cache = cache.lock().unwrap();
    cache.refresh()
}

pub fn inject_devices(
    oci_spec: &mut Spec,
    devices: Vec<String>,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
    let cache = get_default_cache();
    let mut cache = cache.lock().unwrap();
    cache.inject_devices(Some(oci_spec), devices)
}

pub fn list_devices() -> Vec<String> {
    let cache = get_default_cache();
    let mut cache = cache.lock().unwrap();
    cache.list_devices()
}

pub fn get_errors() -> HashMap<String, Vec<anyhow::Error>> {
    let cache = get_default_cache();
    let cache = cache.lock().unwrap();
    cache.get_errors()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_dirs::with_spec_dirs;
    use std::fs;

    // One test covers the whole lifecycle: the cache is a process-wide
    // singleton, so independent tests would race each other's state.
    #[test]
    fn default_cache_is_a_singleton_and_configurable() {
        let first = get_default_cache();
        let second = get_default_cache();
        assert!(
            Arc::ptr_eq(&first, &second),
            "get_default_cache must return the same instance"
        );

        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("vendor.yaml"),
            r#"cdiVersion: "0.6.0"
kind: "vendor.com/device"
devices:
  - name: "gpu0"
    containerEdits:
      env:
        - "VENDOR=1"
"#,
        )
        .unwrap();

        configure(vec![with_spec_dirs(&[dir.path().to_str().unwrap()])]).unwrap();
        assert_eq!(
            first.lock().unwrap().spec_dirs,
            vec![dir.path().to_str().unwrap().to_string()],
            "configure must act on the singleton"
        );

        // Empty configure is a no-op, not an error.
        configure(vec![]).unwrap();

        refresh().unwrap();
        assert_eq!(list_devices(), vec!["vendor.com/device=gpu0".to_string()]);
        assert!(get_errors().is_empty());

        let mut oci_spec = Spec::default();
        inject_devices(&mut oci_spec, vec!["vendor.com/device=gpu0".to_string()]).unwrap();
        let env = oci_spec.process().as_ref().unwrap().env().as_ref().unwrap();
        assert!(env.contains(&"VENDOR=1".to_string()));
    }
}
