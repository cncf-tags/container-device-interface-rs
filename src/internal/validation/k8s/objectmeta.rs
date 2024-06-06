use anyhow::{Error, Result};
use std::collections::BTreeMap;

const TOTAL_ANNOTATION_SIZE_LIMIT: usize = 256 * 1024; // 256 kB

use super::validation::is_qualified_name;

pub fn validate_annotations(annotations: &BTreeMap<String, String>, path: &str) -> Result<()> {
    let mut errs = Vec::new();

    for k in annotations.keys() {
        for msg in is_qualified_name(&k.to_lowercase()) {
            errs.push(format!("{}.{} is invalid: {}", path, k, msg));
        }
    }

    if let Err(err) = validate_annotations_size(annotations) {
        errs.push(format!("{} is too long: {}", path, err));
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(Error::msg(errs.join(", ")))
    }
}

fn validate_annotations_size(annotations: &BTreeMap<String, String>) -> Result<()> {
    let total_size: usize = annotations.iter().map(|(k, v)| k.len() + v.len()).sum();

    if total_size > TOTAL_ANNOTATION_SIZE_LIMIT {
        Err(Error::msg(format!(
            "annotations size {} is larger than limit {}",
            total_size, TOTAL_ANNOTATION_SIZE_LIMIT
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::validate::validate_spec_annotations;
    use super::validate_annotations;

    use std::collections::BTreeMap;

    #[test]
    fn test_validate_annotations() {
        let mut annotations = BTreeMap::new();
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
        let path = "test.annotations";

        assert!(validate_annotations(&annotations, path).is_ok());

        let mut large_annotations = BTreeMap::new();
        let long_value = "CDI".repeat(super::TOTAL_ANNOTATION_SIZE_LIMIT + 1);
        large_annotations.insert("CDIKEY".to_string(), long_value);
        assert!(validate_annotations(&large_annotations, path).is_err());

        let mut invalid_annotations = BTreeMap::new();
        invalid_annotations.insert("inv$$alid_CDIKEY".to_string(), "inv$$alid_CDIVAL".to_string());
        assert!(validate_annotations(&invalid_annotations, path).is_err());
    }

    #[test]
    fn test_validate_spec_annotations() {
        let mut annotations = BTreeMap::new();
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

        let mut large_annotations = BTreeMap::new();
        let long_value = "CDI".repeat(super::TOTAL_ANNOTATION_SIZE_LIMIT + 1);
        large_annotations.insert("CDIKEY".to_string(), long_value);
        assert!(validate_spec_annotations("", &large_annotations).is_err());

        let mut invalid_annotations = BTreeMap::new();
        invalid_annotations.insert("inva$$lid_CDIKEY".to_string(), "inval$$id_CDIVAL".to_string());
        assert!(validate_spec_annotations("CDITEST", &invalid_annotations).is_err());
    }
}
