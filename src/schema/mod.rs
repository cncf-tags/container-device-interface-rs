use std::{collections::BTreeMap, path::Path};

use anyhow::{anyhow, Context, Result};
use jsonschema::{Draft, Validator};
use once_cell::sync::Lazy;
use serde_json::Value;

use crate::{
    internal::validation::validate::validate_spec_annotations, spec::SpecValidator,
    specs::config::Spec as CDISpec, version::validate_declared_version_fields,
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

// SchemaValidator validates CDI Specs against a JSON schema. Install it
// with spec::set_spec_validator to check every Spec as it is loaded, the
// equivalent of Go's cdi.SetSpecValidator(schema.WithSchema(s)); the cdi
// and validate CLIs do this for their --schema argument.
pub struct SchemaValidator {
    source: SchemaSource,
}

enum SchemaSource {
    // The embedded CDI schema (schema.json plus defs.json).
    Builtin,
    // A CDI schema.json compiled with its defs.json; documents also get the
    // CDI content checks (validate_cdi).
    Cdi(Validator),
    // Any other JSON schema; documents are checked against it alone.
    Generic(Validator),
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::builtin()
    }
}

impl std::fmt::Debug for SchemaValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source = match self.source {
            SchemaSource::Builtin => "builtin",
            SchemaSource::Cdi(_) => "cdi",
            SchemaSource::Generic(_) => "generic",
        };
        f.debug_struct("SchemaValidator")
            .field("source", &source)
            .finish()
    }
}

impl SchemaValidator {
    // builtin validates against the embedded CDI schema.
    pub fn builtin() -> Self {
        Self {
            source: SchemaSource::Builtin,
        }
    }

    // from_cdi_schema compiles a CDI schema.json together with its defs.json.
    pub fn from_cdi_schema(schema_data: &[u8], defs_data: &[u8]) -> Result<Self> {
        Ok(Self {
            source: SchemaSource::Cdi(compile_cdi_schema(schema_data, defs_data)?),
        })
    }

    // from_file compiles the schema file at path. A sibling defs.json is
    // used when present; without one the schema must not reference defs.json.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let schema_path = path.as_ref();
        let schema_data = std::fs::read(schema_path)
            .with_context(|| format!("read schema {}", schema_path.display()))?;

        let defs_path = schema_path.with_file_name("defs.json");
        if defs_path.exists() {
            let defs_data = std::fs::read(&defs_path)
                .with_context(|| format!("read {}", defs_path.display()))?;
            return Self::from_cdi_schema(&schema_data, &defs_data);
        }

        let schema_json: Value = serde_json::from_slice(&schema_data)
            .with_context(|| format!("parse schema {}", schema_path.display()))?;
        if refs_defs_json(&schema_json) {
            return Err(anyhow!(
                "schema {} references defs.json, but no sibling defs.json was found",
                schema_path.display()
            ));
        }
        let validator = Validator::options()
            .with_draft(Draft::Draft7)
            .build(&schema_json)
            .with_context(|| format!("compile schema {}", schema_path.display()))?;
        Ok(Self {
            source: SchemaSource::Generic(validator),
        })
    }

    // validate_document validates a JSON or YAML document.
    pub fn validate_document(&self, doc_data: &[u8]) -> Result<()> {
        match &self.source {
            SchemaSource::Builtin => validate_builtin(doc_data),
            SchemaSource::Cdi(schema) => validate_cdi(schema, doc_data),
            SchemaSource::Generic(schema) => validate(schema, doc_data),
        }
    }
}

impl SpecValidator for SchemaValidator {
    fn validate_spec(&self, raw_spec: &CDISpec) -> Result<()> {
        let data =
            serde_yaml::to_string(raw_spec).context("marshal CDI spec for schema validation")?;
        self.validate_document(data.as_bytes())
    }
}

// load resolves a schema source the way Go's schema.Load does: "builtin"
// selects the embedded CDI schema, "none" (or an empty string) disables
// schema validation, anything else is a schema file path.
pub fn load(source: &str) -> Result<Option<SchemaValidator>> {
    match source {
        "builtin" => Ok(Some(SchemaValidator::builtin())),
        "none" | "" => Ok(None),
        path => SchemaValidator::from_file(path).map(Some),
    }
}

fn refs_defs_json(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get("$ref")
                .and_then(Value::as_str)
                .is_some_and(|reference| {
                    reference == "defs.json" || reference.starts_with("defs.json#")
                })
                || object.values().any(refs_defs_json)
        }
        Value::Array(values) => values.iter().any(refs_defs_json),
        _ => false,
    }
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
    use crate::spec::{
        clear_spec_validator, new_spec, set_spec_validator, SPEC_VALIDATOR_TEST_LOCK,
    };
    use std::{path::PathBuf, sync::PoisonError};

    const GOOD_DOC: &[u8] = br#"
cdiVersion: "1.1.0"
kind: "vendor.com/device"
devices:
  - name: "gpu0"
    containerEdits:
      deviceNodes:
        - path: "/dev/null"
"#;

    #[test]
    fn schema_validator_checks_typed_specs() {
        let good: CDISpec = serde_yaml::from_slice(GOOD_DOC).expect("good document parses");
        SchemaValidator::builtin()
            .validate_spec(&good)
            .expect("valid spec passes the builtin schema");
        SchemaValidator::default()
            .validate_document(GOOD_DOC)
            .expect("default is the builtin schema");

        let mut bad = good.clone();
        bad.annotations
            .insert("inva$$lid_CDIKEY".to_string(), "value".to_string());
        let err = SchemaValidator::builtin()
            .validate_spec(&bad)
            .expect_err("invalid annotation key is rejected");
        assert!(err.to_string().contains("annotations"), "{err}");
    }

    #[test]
    fn installed_schema_validator_runs_when_specs_load() {
        let _lock = SPEC_VALIDATOR_TEST_LOCK
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let good: CDISpec = serde_yaml::from_slice(GOOD_DOC).expect("good document parses");

        set_spec_validator(SchemaValidator::builtin());
        let loaded = new_spec(&good, &PathBuf::from("/tmp/vendor-device.yaml"), 0);
        clear_spec_validator();

        loaded.expect("builtin schema accepts a valid spec on load");
    }

    #[test]
    fn schema_validator_debug_names_the_source() {
        assert_eq!(
            format!("{:?}", SchemaValidator::builtin()),
            "SchemaValidator { source: \"builtin\" }"
        );
    }

    #[test]
    fn from_file_reports_unreadable_and_invalid_schemas() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.json");
        let schema_arg = schema_path.to_str().unwrap();

        std::fs::write(&schema_path, b"{ not json").unwrap();
        let err = load(schema_arg).expect_err("unparsable schema fails");
        assert!(format!("{err:#}").contains("parse schema"), "{err:#}");

        std::fs::write(&schema_path, r#"{"type": 123}"#).unwrap();
        let err = load(schema_arg).expect_err("invalid schema fails to compile");
        assert!(format!("{err:#}").contains("compile schema"), "{err:#}");

        // A sibling defs.json that exists but is not a readable file.
        std::fs::create_dir(dir.path().join("defs.json")).unwrap();
        std::fs::write(&schema_path, SCHEMA_JSON).unwrap();
        let err = load(schema_arg).expect_err("unreadable defs.json fails");
        assert!(format!("{err:#}").contains("defs.json"), "{err:#}");
    }

    #[test]
    fn load_resolves_builtin_none_and_schema_files() {
        assert!(load("builtin").unwrap().is_some());
        assert!(load("none").unwrap().is_none());
        assert!(load("").unwrap().is_none());

        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("schema.json");
        let schema_arg = schema_path.to_str().unwrap();

        assert!(
            load(dir.path().join("missing.json").to_str().unwrap()).is_err(),
            "missing schema file fails"
        );

        // Standalone schema: applied as-is, no CDI content checks.
        std::fs::write(&schema_path, r#"{"type": "object"}"#).unwrap();
        let generic = load(schema_arg)
            .unwrap()
            .expect("file source yields a validator");
        generic
            .validate_document(b"devices: []\n")
            .expect("generic schema applies alone");
        assert_eq!(
            format!("{generic:?}"),
            "SchemaValidator { source: \"generic\" }"
        );

        // Dangling defs.json reference (nested in an array) without a
        // sibling defs.json.
        std::fs::write(
            &schema_path,
            r##"{"type": "object", "allOf": [{"$ref": "defs.json#/definitions/x"}]}"##,
        )
        .unwrap();
        let err = load(schema_arg).expect_err("dangling defs.json reference fails");
        assert!(err.to_string().contains("no sibling defs.json"), "{err}");

        // CDI schema with its defs.json: CDI content checks apply.
        std::fs::write(&schema_path, SCHEMA_JSON).unwrap();
        std::fs::write(dir.path().join("defs.json"), DEFS_JSON).unwrap();
        let cdi = load(schema_arg)
            .unwrap()
            .expect("CDI schema yields a validator");
        cdi.validate_document(GOOD_DOC)
            .expect("good document passes the CDI schema file");
        assert_eq!(format!("{cdi:?}"), "SchemaValidator { source: \"cdi\" }");
        let empty = b"cdiVersion: \"1.1.0\"\nkind: \"vendor.com/device\"\ndevices: []\n";
        assert!(
            cdi.validate_document(empty).is_err(),
            "empty devices rejected"
        );
    }

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
