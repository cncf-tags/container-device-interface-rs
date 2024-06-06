use std::{
    collections::HashMap,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use path_clean::clean;

use crate::{
    cache::Cache,
    spec::{read_spec, Spec},
    utils::is_cdi_spec,
};

// DEFAULT_STATIC_DIR is the default directory for static CDI Specs.
const DEFAULT_STATIC_DIR: &str = "/etc/cdi";
// DEFAULT_DYNAMIC_DIR is the default directory for generated CDI Specs
const DEFAULT_DYNAMIC_DIR: &str = "/var/run/cdi";

lazy_static! {
    // DEFAULT_SPEC_DIRS is the default Spec directory configuration.
    // While altering this variable changes the package defaults,
    // the preferred way of overriding the default directories is
    // to use a WithSpecDirs options. Otherwise the change is only
    // effective if it takes place before creating the Registry or
    // other Cache instances.
    pub static ref DEFAULT_SPEC_DIRS: &'static [&'static str] = &[
        DEFAULT_STATIC_DIR,
        DEFAULT_DYNAMIC_DIR,
    ];
}

// CdiOption is an option to change some aspect of default CDI behavior.
// We define the CdiOption type using a type alias, which is a Box<dyn FnOnce(&mut Cache)>.
// This means that CdiOption is a trait object that represents a one-time closure that takes a &mut Cache parameter.
type CdiOption = Box<dyn FnOnce(&mut Cache)>;

#[derive(Debug)]
pub struct SpecError {
    message: String,
}

impl SpecError {
    pub fn new(info: &str) -> Self {
        Self {
            message: info.to_owned(),
        }
    }
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "spec error message {}", self.message)
    }
}

impl Error for SpecError {}

pub fn convert_errors(
    spec_errors: &HashMap<String, Vec<Box<dyn Error>>>,
) -> HashMap<String, Vec<Box<dyn Error + Send + Sync + 'static>>> {
    spec_errors
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                value
                    .iter()
                    .map(|error| {
                        Box::new(SpecError::new(&error.to_string()))
                            as Box<dyn Error + Send + Sync + 'static>
                    })
                    .collect(),
            )
        })
        .collect()
}

/// with_spec_dirs returns an option to override the CDI Spec directories.
pub fn with_spec_dirs(dirs: &[&str]) -> CdiOption {
    let cleaned_dirs: Vec<String> = dirs
        .iter()
        .map(|dir| {
            clean(PathBuf::from(*dir))
                .into_os_string()
                .into_string()
                .unwrap()
        })
        .collect();

    Box::new(move |cache: &mut Cache| {
        cache.spec_dirs.clone_from(&cleaned_dirs);
    })
}

#[allow(dead_code)]
fn traverse_dir<F>(dir_path: &Path, traverse_fn: &mut F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Path) -> Result<(), Box<dyn Error>>,
{
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                traverse_dir(&path, traverse_fn)?;
            } else {
                traverse_fn(&path)?;
            }
        }
    }
    Ok(())
}

// scan_spec_dirs scans the given directories looking for CDI Spec files,
// which are all files with a '.json' or '.yaml' suffix. For every Spec
// file discovered, if it's a cdi spec, then loads a Spec from the file
// with the priority (the index of the directory in the slice of directories given),
// then collect the CDI Specs, and any error encountered while loading the Spec return Error.
#[allow(dead_code)]
pub(crate) fn scan_spec_dirs<P: AsRef<Path>>(dirs: &[P]) -> Result<Vec<Spec>, Box<dyn Error>> {
    let mut scaned_specs = Vec::new();
    for (priority, dir) in dirs.iter().enumerate() {
        let dir_path = dir.as_ref();
        if !dir_path.is_dir() {
            continue;
        }

        let mut operation = |path: &Path| -> Result<(), Box<dyn Error>> {
            if !path.is_dir() && is_cdi_spec(path) {
                let spec = match read_spec(&path.to_path_buf(), priority as i32) {
                    Ok(spec) => spec,
                    Err(err) => {
                        return Err(Box::new(SpecError::new(&err.to_string())));
                    }
                };
                scaned_specs.push(spec);
            }
            Ok(())
        };

        if let Err(e) = traverse_dir(dir_path, &mut operation) {
            return Err(Box::new(SpecError::new(&e.to_string())));
        }
    }

    Ok(scaned_specs)
}

#[cfg(test)]
mod tests {
    //TODO
}
