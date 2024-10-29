use anyhow::{anyhow, Context, Result};
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
pub(crate) fn update_annotations(
    option_annotations: Option<HashMap<String, String>>,
    plugin_name: &str,
    device_id: &str,
    devices: Vec<String>,
) -> Result<HashMap<String, String>> {
    let mut annotations = option_annotations.unwrap_or_else(HashMap::new);

    let key = annotation_key(plugin_name, device_id).context("CDI annotation key failed")?;
    if annotations.contains_key(&key) {
        return Err(anyhow!("CDI annotation key collision, key {:?} used", key));
    }
    let value = annotation_value(devices).context("CDI annotation value failed")?;

    annotations.insert(key, value);

    Ok(annotations.clone())
}

// ParseAnnotations parses annotations for CDI device injection requests.
// The keys and devices from all such requests are collected into slices
// which are returned as the result. All devices are expected to be fully
// qualified CDI device names. If any device fails this check empty slices
// are returned along with a non-nil error. The annotations are expected
// to be formatted by, or in a compatible fashion to UpdateAnnotations().
#[allow(dead_code)]
pub fn parse_annotations(
    annotations: &HashMap<String, String>,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut keys: Vec<String> = Vec::new();
    let mut devices: Vec<String> = Vec::new();

    for (k, v) in annotations {
        if !k.starts_with(ANNOTATION_PREFIX) {
            continue;
        }

        for device in v.split(',') {
            if !parser::is_qualified_name(device) {
                return Err(anyhow!("invalid CDI device name {}", device));
            }
            devices.push(device.to_string());
        }
        keys.push(k.to_string());
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

    let name = format!(
        "{}_{}",
        plugin_name.to_owned(),
        &device_id.replace('/', "_")
    );
    if name.len() > MAX_NAME_LEN {
        return Err(anyhow!("invalid plugin+deviceID {:?}, too long", name));
    }

    if !name.starts_with(|c: char| c.is_alphanumeric()) {
        return Err(anyhow!(
            "invalid name {:?}, first '{}' should be alphanumeric",
            name.as_str(),
            name.chars().next().unwrap(),
        ));
    }

    if !name
        .chars()
        .skip(1)
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(anyhow!(
            "invalid name {:?}, invalid character '{}'",
            name.as_str(),
            name.chars()
                .find(|c| !c.is_alphanumeric() && *c != '_' && *c != '-' && *c != '.')
                .unwrap(),
        ));
    }

    if !name.ends_with(|c: char| c.is_alphanumeric()) {
        return Err(anyhow!(
            "invalid name {:?}, last '{}' should be alphanumeric",
            name.as_str(),
            name.chars().next_back().unwrap(),
        ));
    }

    Ok(format!("{}{}", ANNOTATION_PREFIX, name))
}

// AnnotationValue returns an annotation value for the given devices.
#[allow(dead_code)]
pub(crate) fn annotation_value(devices: Vec<String>) -> Result<String> {
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
    use std::collections::HashMap;

    use crate::annotations::{
        annotation_key, annotation_value, parse_annotations, ANNOTATION_PREFIX,
    };

    #[test]
    fn test_parse_annotations() {
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

        // one vendor, multiple devices
        cdi_devices.insert(
            "vendor.class_device".to_string(),
            "vendor.com/class=device1,vendor.com/class=device2,vendor.com/class=device3"
                .to_string(),
        );

        assert!(parse_annotations(&cdi_devices).is_ok());
        let (keys, devices) = parse_annotations(&cdi_devices).unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(devices.len(), 3);
    }

    #[test]
    fn test_annotation_value() {
        let devices = vec![
            "nvidia.com/gpu=0".to_string(),
            "nvidia.com/gpu=1".to_string(),
        ];

        assert!(annotation_value(devices.clone()).is_ok());
        assert_eq!(
            annotation_value(devices.clone()).unwrap(),
            "nvidia.com/gpu=0,nvidia.com/gpu=1"
        );
    }

    #[test]
    fn test_annotation_key() {
        struct TestCase {
            plugin_name: String,
            device_id: String,
            key_result: String,
        }

        let test_cases = vec![
            // valid, with special characters
            TestCase {
                plugin_name: "v-e.n_d.or.cl-as_s".to_owned(),
                device_id: "d_e-v-i-c_e".to_owned(),
                key_result: format!(
                    "{}{}_{}",
                    ANNOTATION_PREFIX, "v-e.n_d.or.cl-as_s", "d_e-v-i-c_e"
                ),
            },
            // valid, with /'s replaced in devID
            TestCase {
                plugin_name: "v-e.n_d.or.cl-as_s".to_owned(),
                device_id: "d-e/v/i/c-e".to_owned(),
                key_result: format!(
                    "{}{}_{}",
                    ANNOTATION_PREFIX, "v-e.n_d.or.cl-as_s", "d-e_v_i_c-e"
                ),
            },
            TestCase {
                // valid, simple
                plugin_name: "vendor.class".to_owned(),
                device_id: "device".to_owned(),
                key_result: format!("{}{}_{}", ANNOTATION_PREFIX, "vendor.class", "device"),
            },
        ];

        for case in test_cases {
            let plugin_name = &case.plugin_name;
            let device_id = &case.device_id;
            assert!(annotation_key(plugin_name, device_id).is_ok());
            assert_eq!(
                annotation_key(plugin_name, device_id).unwrap(),
                case.key_result.clone()
            );
        }

        let test_cases_err = vec![
            // invalid, non-alphanumeric first character
            TestCase {
                plugin_name: "_vendor.class".to_owned(),
                device_id: "device".to_owned(),
                key_result: "".to_owned(),
            },
            // "invalid, non-alphanumeric last character"
            TestCase {
                plugin_name: "vendor.class".to_owned(),
                device_id: "device_".to_owned(),
                key_result: "".to_owned(),
            },
            // invalid, plugin contains invalid characters
            TestCase {
                plugin_name: "ven.dor-cl+ass".to_owned(),
                device_id: "d_e-v-i-c_e".to_owned(),
                key_result: "".to_owned(),
            },
            // "invalid, devID contains invalid characters"
            TestCase {
                plugin_name: "vendor.class".to_owned(),
                device_id: "dev+ice".to_owned(),
                key_result: "".to_owned(),
            },
            // invalid, too plugin long
            TestCase {
                plugin_name: "123456789012345678901234567890123456789012345678901234567".to_owned(),
                device_id: "device".to_owned(),
                key_result: "".to_owned(),
            },
        ];

        for case in test_cases_err {
            let plugin_name = &case.plugin_name;
            let device_id = &case.device_id;
            assert!(annotation_key(plugin_name, device_id).is_err());
        }
    }
}
