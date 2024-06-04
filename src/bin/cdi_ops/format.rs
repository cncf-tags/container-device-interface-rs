use serde::Serialize;
use serde_json;
use serde_yaml;
use std::path::Path;

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
                out.push_str(&indent(level));
                out.push_str(line);
                out.push('\n');
            }
            out
        }
        Err(err) => format!("{}<failed to dump object: {:?}\n", indent(level), err),
    }
}

pub fn indent(level: usize) -> String {
    format!("{:width$}", "", width = level)
}
