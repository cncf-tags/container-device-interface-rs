use std::{collections::HashSet, fs::File, io::BufReader, path::Path};

use anyhow::{anyhow, Context, Result};
use oci_spec::runtime as oci;

pub(crate) fn find_target_devices(devices: Vec<String>, patterns: Vec<String>) -> Vec<String> {
    let devices_set: HashSet<String> = devices.into_iter().collect();

    let mut device_matches: HashSet<String> = HashSet::new();

    for pattern in patterns {
        if devices_set.contains(&pattern) {
            device_matches.insert(pattern);
        }
    }

    let mut devices = device_matches.into_iter().collect::<Vec<String>>();
    devices.sort();

    devices
}

pub fn read_oci_spec(path: &str) -> Result<oci::Spec> {
    if !Path::new(path).exists() {
        return Err(anyhow!("path of oci spec not found"));
    }

    let ocispec_file = BufReader::new(File::open(path)?);
    let oci_spec: oci::Spec = serde_yaml::from_reader(ocispec_file).context("read config")?;

    Ok(oci_spec)
}

#[cfg(test)]

mod tests {
    use crate::cdi_ops::utils::find_target_devices;

    #[test]
    fn test_find_target_devices() {
        let devices = vec![
            "device1".to_string(),
            "device2".to_string(),
            "device3".to_string(),
        ];
        let patterns = vec!["device2".to_string(), "device4".to_string()];

        let matches = find_target_devices(devices, patterns);

        println!("Matches: {:?}", matches);
    }
}
