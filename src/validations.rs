use std::collections::HashMap;

use anyhow::Result;

use crate::parser::parse_qualified_name;

const TOTAL_ANNOTATION_SIZE_LIMIT: usize = 256 * 1024; // 256 kB

#[allow(dead_code)]
pub fn validate_annotations(annotations: &HashMap<String, String>) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let total_size: usize = annotations.iter().map(|(k, v)| k.len() + v.len()).sum();

    if total_size > TOTAL_ANNOTATION_SIZE_LIMIT {
        errors.push(format!(
            "annotations size {} is larger than limit {}",
            total_size, TOTAL_ANNOTATION_SIZE_LIMIT
        ));
    }

    for (key, value) in annotations {
        if let Err(msg) = parse_qualified_name(value) {
            errors.push(format!("{}:{} is invalid: {}", key, value, msg));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[allow(dead_code)]
pub fn validate_spec_annotations(
    name: &str,
    annotations: &HashMap<String, String>,
) -> Result<(), Vec<String>> {
    let path = if name.is_empty() {
        "annotations".to_string()
    } else {
        format!("{}.annotations", name)
    };

    validate_annotations(annotations).map_err(|mut errors| {
        errors.iter_mut().for_each(|error| {
            error.insert_str(0, &format!("{}: ", path));
        });
        errors
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_annotations() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "cdi.k8s.io/vfio17".to_string(),
            "nvidia.com/gpu=0".to_string(),
        );
        annotations.insert(
            "cdi.k8s.io/vfio18".to_string(),
            "nvidia.com/gpu=1".to_string(),
        );
        annotations.insert(
            "cdi.k8s.io/vfio19".to_string(),
            "nvidia.com/gpu=all".to_string(),
        );
        assert!(validate_annotations(&annotations).is_ok());

        let mut large_annotations = HashMap::new();
        let long_value = "CDI".repeat(TOTAL_ANNOTATION_SIZE_LIMIT + 1);
        large_annotations.insert("CDIKEY".to_string(), long_value);
        assert!(validate_annotations(&large_annotations).is_err());

        let mut invalid_annotations = HashMap::new();
        invalid_annotations.insert("invalid_CDIKEY".to_string(), "invalied_CDIVAL".to_string());
        assert!(validate_annotations(&invalid_annotations).is_err());
    }

    #[test]
    fn test_validate_spec_annotations() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "cdi.k8s.io/vfio17".to_string(),
            "nvidia.com/gpu=0".to_string(),
        );
        annotations.insert(
            "cdi.k8s.io/vfio18".to_string(),
            "nvidia.com/gpu=1".to_string(),
        );
        annotations.insert(
            "cdi.k8s.io/vfio19".to_string(),
            "nvidia.com/gpu=all".to_string(),
        );
        assert!(validate_spec_annotations("", &annotations).is_ok());
        assert!(validate_spec_annotations("CDITEST", &annotations).is_ok());

        let mut large_annotations = HashMap::new();
        let long_value = "CDI".repeat(TOTAL_ANNOTATION_SIZE_LIMIT + 1);
        large_annotations.insert("CDIKEY".to_string(), long_value);
        assert!(validate_spec_annotations("", &large_annotations).is_err());

        let mut invalid_annotations = HashMap::new();
        invalid_annotations.insert("invalid_CDIKEY".to_string(), "invalied_CDIVAL".to_string());
        assert!(validate_spec_annotations("CDITEST", &invalid_annotations).is_err());
    }
}
