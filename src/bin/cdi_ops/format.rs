use serde::Serialize;
use serde_json;
use serde_yaml;
use std::path::Path;
use std::fmt::Write;
use std::error::Error;

pub fn choose_format(format: &str, path: &str) -> String {
    let mut format = format.to_string();
    if format.is_empty() {
        if let Some(ext) = Path::new(path).extension() {
            if ext == "json" || ext == "yaml" {
                format = ext.to_string_lossy().to_string();
            }
        }
    }
    format
}

pub fn marshal_object<T: Serialize>(level: usize, obj: &T, format: &str) -> String {
    let raw_result: Result<String, Box<dyn Error>> = if format == "json" {
        serde_json::to_string_pretty(obj).map_err(|e| Box::new(e) as Box<dyn Error>)
    } else {
        serde_yaml::to_string(obj).map_err(|e| Box::new(e) as Box<dyn Error>)
    };

    match raw_result {
        Ok(data) => {
            let mut out = String::new();
            for line in data.lines() {
		writeln!(out, "{}{}", &indent(level), line).unwrap();
            }
            out
        }
        Err(err) => format!("{}<failed to dump object: {:?}\n", indent(level), err),
    }
}

pub fn indent(level: usize) -> String {
    format!("{:width$}", "", width = level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestObjMarshal {
        name: String,
        index: u32,
    }

    #[derive(Serialize)]
    struct NestedObjMarshal {
        outer_name: String,
        inner: TestObjMarshal,
    }

    #[test]
    fn test_marshal_object_json() {
        let obj = TestObjMarshal {
            name: String::from("TestJson"),
            index: 30,
        };
        let expected = r#"{
  "name": "TestJson",
  "index": 30
}"#;
        let result = marshal_object(0, &obj, "json");
        assert_eq!(result.trim(), expected);
    }

    #[test]
    fn test_marshal_object_yaml() {
        let obj = TestObjMarshal {
            name: String::from("TestYaml"),
            index: 30,
        };
        let expected = r#"name: TestYaml
index: 30"#;
        let result = marshal_object(0, &obj, "yaml");
        assert_eq!(result.trim(), expected);
    }

    #[test]
    fn test_nested_marshal_object_json() {
        let obj = NestedObjMarshal {
            outer_name: String::from("Outer"),
            inner: TestObjMarshal {
                name: String::from("Inner"),
                index: 25,
            },
        };
        let expected = r#"{
  "outer_name": "Outer",
  "inner": {
    "name": "Inner",
    "index": 25
  }
}"#;
        let result = marshal_object(0, &obj, "json");
        assert_eq!(result.trim(), expected);
    }

    #[test]
    fn test_nested_marshal_object_yaml() {
        let obj = NestedObjMarshal {
            outer_name: String::from("Outer"),
            inner: TestObjMarshal {
                name: String::from("Inner"),
                index: 25,
            },
        };
        let expected = r#"outer_name: Outer
inner:
  name: Inner
  index: 25"#;

        let result = marshal_object(0, &obj, "yaml");
        assert_eq!(result.trim(), expected);
    }

    #[test]
    fn test_marshal_object_json_with_indent() {
        let obj = TestObjMarshal {
            name: String::from("TestJson"),
            index: 20,
        };
        let expected = r#"{
   "name": "TestJson",
   "index": 20
 }"#;
        let result = marshal_object(1, &obj, "json");
        assert_eq!(result.trim(), expected);
    }

    #[test]
    fn test_marshal_object_yaml_with_indent() {
        let obj = TestObjMarshal {
            name: String::from("TestYaml"),
            index: 30,
        };
        let expected = r#"name: TestYaml
 index: 30"#;
        let result = marshal_object(1, &obj, "yaml");
        assert_eq!(result.trim(), expected);
    }

    #[test]
    fn test_marshal_object_json_without_indent() {
        let obj = TestObjMarshal {
            name: String::from("TestJson"),
            index: 10,
        };
        let result = marshal_object(0, &obj, "json");
        let expected = r#"
{
  "name": "TestJson",
  "index": 10
}
"#;
        assert_eq!(result.trim(), expected.trim());
    }

    #[test]
    fn test_marshal_object_yaml_without_indent() {
        let obj = TestObjMarshal {
            name: String::from("TestYaml"),
            index: 35,
        };
        let result = marshal_object(0, &obj, "yaml");
        let expected = r#"
name: TestYaml
index: 35
"#;
        assert_eq!(result.trim(), expected.trim());
    }
}
