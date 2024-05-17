use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::vec::Vec;

use crate::parser;

const ANNOTATION_PREFIX: &str = "cdi.k8s.io/";
const MAX_NAME_LEN: usize = 63;

// UpdateAnnotations updates annotations with a plugin-specific CDI device
// injection request for the given devices. Upon any error a non-nil error
// is returned and annotations are left intact. By convention plugin should
// be in the format of "vendor.device-type".
#[allow(dead_code)]
pub(crate) fn update() {
    println!("cdi::annotations::update");
}

// ParseAnnotations parses annotations for CDI device injection requests.
// The keys and devices from all such requests are collected into slices
// which are returned as the result. All devices are expected to be fully
// qualified CDI device names. If any device fails this check empty slices
// are returned along with a non-nil error. The annotations are expected
// to be formatted by, or in a compatible fashion to UpdateAnnotations().
#[allow(dead_code)]
pub(crate) fn parse_annotations(
    annotations: HashMap<String, String>,
) -> Result<(Vec<String>, Vec<String>), anyhow::Error> {
    let mut keys: Vec<String> = Vec::new();
    let mut devices: Vec<String> = Vec::new();

    for (k, v) in annotations {
        if !k.starts_with(ANNOTATION_PREFIX) {
            continue;
        }

        for device in v.split(',') {
            if let Err(_e) = parser::parse_qualified_name(device) {
                return Err(anyhow!("invalid CDI device name {}", device));
            }
            devices.push(device.to_string());
        }
        keys.push(k);
    }
    Ok((keys, devices))
}

// AnnotationKey returns a unique annotation key for an device allocation
// by a K8s device plugin. pluginName should be in the format of
// "vendor.device-type". deviceID is the ID of the device the plugin is
// allocating. It is used to make sure that the generated key is unique
// even if multiple allocations by a single plugin needs to be annotated.
#[allow(dead_code)]
pub(crate) fn annotation_key(plugin_name: &str, device_id: &str) -> Result<String> {
    if plugin_name.is_empty() {
        return Err(anyhow!("invalid plugin name, empty"));
    }
    if device_id.is_empty() {
        return Err(anyhow!("invalid deviceID, empty"));
    }

    let name = plugin_name.to_owned() + "_" + &device_id.replace('/', "_");

    if name.len() > MAX_NAME_LEN {
        return Err(anyhow!("invalid plugin+deviceID {:?}, too long", name));
    }

    if let Some(first) = name.chars().next() {
        if !first.is_alphanumeric() {
            return Err(anyhow!(
                "invalid name {:?}, first '{}' should be alphanumeric",
                name,
                first
            ));
        }
    }

    if name.len() > 2 {
        for c in name[1..name.len() - 1].chars() {
            match c {
                c if c.is_alphanumeric() => {}
                '_' | '-' | '.' => {}
                _ => {
                    return Err(anyhow!(
                        "invalid name {:?}, invalid character '{}'",
                        name,
                        c
                    ))
                }
            }
        }
    }

    if let Some(last) = name.chars().last() {
        if !last.is_alphanumeric() {
            return Err(anyhow!(
                "invalid name {:?}, last '{}' should be alphanumeric",
                name,
                last
            ));
        }
    }

    Ok(ANNOTATION_PREFIX.to_string() + &name)
}

// AnnotationValue returns an annotation value for the given devices.
#[allow(dead_code)]
pub(crate) fn annotation_value(devices: Vec<String>) -> Result<String, anyhow::Error> {
    devices.iter().try_for_each(|device| {
        // Assuming parser::parse_qualified_name expects a &String and returns Result<(), Error>
        match crate::parser::parse_qualified_name(device) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    })?;

    let device_strs: Vec<&str> = devices.iter().map(AsRef::as_ref).collect();
    let value = device_strs.join(",");

    Ok(value)
}

#[cfg(test)]

mod tests {
    use crate::annotations;
    use std::collections::HashMap;

    #[test]
    fn parse_annotations() {
        let mut cdi_devices = HashMap::new();

        cdi_devices.insert(
            "cdi.k8s.io/vfio17".to_string(),
            "nvidia.com/gpu=0".to_string(),
        );
        cdi_devices.insert(
            "cdi.k8s.io/vfio18".to_string(),
            "nvidia.com/gpu=1".to_string(),
        );
        cdi_devices.insert(
            "cdi.k8s.io/vfio19".to_string(),
            "nvidia.com/gpu=all".to_string(),
        );

        match annotations::parse_annotations(cdi_devices) {
            Ok((keys, devices)) => {
                assert_eq!(keys.len(), 3);
                assert_eq!(devices.len(), 3);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    #[test]
    fn annotation_value() {
        let devices = vec![
            "nvidia.com/gpu=0".to_string(),
            "nvidia.com/gpu=1".to_string(),
        ];
        match annotations::annotation_value(devices) {
            Ok(value) => {
                assert_eq!(value, "nvidia.com/gpu=0,nvidia.com/gpu=1");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    #[test]
    fn annotation_key() {
        let plugin_name = "nvida-device-plugin";
        let device_id = "gpu=0";
        match annotations::annotation_key(plugin_name, device_id) {
            Ok(key) => {
                assert_eq!(key, "nvidia-device-plugin_gpu=0");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
