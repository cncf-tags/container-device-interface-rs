use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use jsonschema::{Draft, Validator};
use once_cell::sync::Lazy;
use serde_json::Value;

use crate::{
    internal::validation::validate::validate_spec_annotations, specs::config::Spec as CDISpec,
    version::validate_declared_version_fields,
};

const SCHEMA_JSON: &str = include_str!("schema.json");
const DEFS_JSON: &str = include_str!("defs.json");
static BUILTIN_SCHEMA: Lazy<Result<Validator, String>> =
    Lazy::new(|| compile_builtin_schema().map_err(|err| format!("{err:#}")));

pub fn builtin_schema_value() -> Result<Value> {
    cdi_schema_value(SCHEMA_JSON.as_bytes(), DEFS_JSON.as_bytes())
}

pub fn cdi_schema_value(schema_data: &[u8], defs_data: &[u8]) -> Result<Value> {
    let mut schema_json: Value =
        serde_json::from_slice(schema_data).context("parse CDI schema.json")?;
    let defs_json: Value = serde_json::from_slice(defs_data).context("parse CDI defs.json")?;
    rewrite_defs_json_refs(&mut schema_json);

    let schema = schema_json
        .as_object_mut()
        .ok_or_else(|| anyhow!("CDI schema must be a JSON object"))?;
    let definitions = defs_json
        .get("definitions")
        .cloned()
        .ok_or_else(|| anyhow!("CDI defs.json must contain definitions"))?;
    schema.insert("definitions".to_string(), definitions);

    Ok(schema_json)
}

fn rewrite_defs_json_refs(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(reference)) = object.get_mut("$ref") {
                if let Some(definition) = reference.strip_prefix("defs.json#/definitions/") {
                    *reference = format!("#/definitions/{definition}");
                }
            }

            for value in object.values_mut() {
                rewrite_defs_json_refs(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                rewrite_defs_json_refs(value);
            }
        }
        _ => {}
    }
}

pub fn compile_builtin_schema() -> Result<Validator> {
    let schema_json = builtin_schema_value()?;
    Validator::options()
        .with_draft(Draft::Draft7)
        .build(&schema_json)
        .context("compile builtin CDI schema")
}

pub fn compile_cdi_schema(schema_data: &[u8], defs_data: &[u8]) -> Result<Validator> {
    let schema_json = cdi_schema_value(schema_data, defs_data)?;
    Validator::options()
        .with_draft(Draft::Draft7)
        .build(&schema_json)
        .context("compile CDI schema")
}

pub fn document_value(doc_data: &[u8]) -> Result<Value> {
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_slice(doc_data).context("parse CDI document")?;
    serde_json::to_value(yaml_value).context("convert CDI document to JSON value")
}

pub fn validate(schema: &Validator, doc_data: &[u8]) -> Result<()> {
    let doc = document_value(doc_data)?;
    validate_value(schema, &doc)
}

pub fn validate_cdi(schema: &Validator, doc_data: &[u8]) -> Result<()> {
    let doc = document_value(doc_data)?;
    validate_value(schema, &doc)?;
    validate_cdi_document_content(&doc)?;
    validate_typed_cdi_document(doc_data)
}

pub fn validate_builtin(doc_data: &[u8]) -> Result<()> {
    let schema = BUILTIN_SCHEMA
        .as_ref()
        .map_err(|err| anyhow!("compile builtin CDI schema: {err}"))?;
    validate_cdi(schema, doc_data)
}

fn validate_value(schema: &Validator, doc: &Value) -> Result<()> {
    let errors: Vec<String> = schema
        .iter_errors(doc)
        .map(|error| error.to_string())
        .collect();

    if errors.is_empty() {
        return Ok(());
    }

    Err(anyhow!("schema validation failed: {}", errors.join("; ")))
}

fn validate_typed_cdi_document(doc_data: &[u8]) -> Result<()> {
    let spec: CDISpec =
        serde_yaml::from_slice(doc_data).context("parse CDI document using declared version")?;
    validate_declared_version_fields(&spec)
}

fn validate_cdi_document_content(doc: &Value) -> Result<()> {
    if doc
        .get("devices")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        return Err(anyhow!(
            "CDI schema validation failed: top-level devices array must not be empty"
        ));
    }

    validate_annotations("", doc.get("annotations"))?;

    if let Some(devices) = doc.get("devices").and_then(Value::as_array) {
        for device in devices {
            let name = device
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            validate_annotations(name, device.get("annotations"))?;
        }
    }

    Ok(())
}

fn validate_annotations(name: &str, annotations: Option<&Value>) -> Result<()> {
    let Some(Value::Object(annotations)) = annotations else {
        return Ok(());
    };

    let mut parsed = BTreeMap::new();
    for (key, value) in annotations {
        let Some(value) = value.as_str() else {
            return Err(anyhow!(
                "invalid annotation {}.{}; annotation value is not a string",
                name,
                key
            ));
        };
        parsed.insert(key.clone(), value.to_string());
    }

    validate_spec_annotations(name, &parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_schema_accepts_v1_1_features() {
        let doc = br#"
cdiVersion: "1.1.0"
kind: "vendor.com/device"
containerEdits:
  netDevices:
    - hostInterfaceName: "eth0"
      name: "container_eth0"
  intelRdt:
    schemata:
      - "L3:0=ffff"
    enableMonitoring: true
devices:
  - name: "gpu0"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#;

        validate_builtin(doc).expect("v1.1.0 document should validate");
    }

    #[test]
    fn builtin_schema_rejects_wrong_type() {
        let doc = br#"
cdiVersion: "1.1.0"
kind: "vendor.com/device"
devices: "not-an-array"
"#;

        assert!(validate_builtin(doc).is_err());
    }

    #[test]
    fn builtin_schema_rejects_v1_1_legacy_intel_rdt_fields() {
        let doc = br#"
cdiVersion: "1.1.0"
kind: "vendor.com/device"
containerEdits:
  intelRdt:
    enableCMT: true
devices:
  - name: "gpu0"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#;

        let err = validate_builtin(doc).expect_err("v1.1.0 must reject legacy Intel RDT fields");

        assert!(err.to_string().contains("enableCMT"));
    }

    #[test]
    fn builtin_schema_rejects_v1_0_intel_rdt_enable_monitoring_field() {
        let doc = br#"
cdiVersion: "1.0.0"
kind: "vendor.com/device"
containerEdits:
  intelRdt:
    enableMonitoring: false
devices:
  - name: "gpu0"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#;

        let err = validate_builtin(doc).expect_err("v1.0.0 must reject v1.1.0 Intel RDT fields");

        assert!(err.to_string().contains("enableMonitoring"));
    }

    #[test]
    fn generic_validate_allows_empty_devices_when_schema_allows_it() {
        let schema_json: Value = serde_json::json!({
            "type": "object"
        });
        let schema = Validator::options()
            .with_draft(Draft::Draft7)
            .build(&schema_json)
            .expect("compile permissive schema");
        let doc = br#"
devices: []
"#;

        validate(&schema, doc).expect("generic validation should only apply the supplied schema");
    }

    #[test]
    fn cdi_schema_rejects_invalid_spec_annotations() {
        let doc = br#"
cdiVersion: "1.1.0"
kind: "vendor.com/device"
annotations:
  "inva$$lid_CDIKEY": "value"
devices:
  - name: "gpu0"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#;

        let err = validate_builtin(doc).expect_err("invalid annotation key should fail");

        assert!(err.to_string().contains("annotations"));
    }

    #[test]
    fn cdi_schema_rejects_invalid_device_annotations() {
        let doc = br#"
cdiVersion: "1.1.0"
kind: "vendor.com/device"
devices:
  - name: "gpu0"
    annotations:
      "inva$$lid_CDIKEY": "value"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#;

        let err = validate_builtin(doc).expect_err("invalid device annotation key should fail");

        assert!(err.to_string().contains("gpu0.annotations"));
    }
}
