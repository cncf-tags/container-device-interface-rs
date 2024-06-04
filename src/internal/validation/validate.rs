use crate::internal::validation::k8s::objectmeta as k8s;
use anyhow::Result;
use std::collections::BTreeMap;

pub fn validate_spec_annotations(name: &str, annotations: &BTreeMap<String, String>) -> Result<()> {
    if annotations.is_empty() {
        return Ok(());
    }
    validate_annotations_inner(name, &annotations)
}

fn validate_annotations_inner(name: &str, annotations: &BTreeMap<String, String>) -> Result<()> {
    let path = if !name.is_empty() {
        format!("{}.annotations", name)
    } else {
        "annotations".to_string()
    };

    k8s::validate_annotations(annotations, &path)
}
