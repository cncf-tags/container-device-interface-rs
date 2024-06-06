use anyhow::{anyhow, Result};

// QualifiedName returns the qualified name for a device.
// The syntax for a qualified device names is
//
//	"<vendor>/<class>=<name>".
//
// A valid vendor and class name may contain the following runes:
//
//	'A'-'Z', 'a'-'z', '0'-'9', '.', '-', '_'.
//
// A valid device name may contain the following runes:
//
//	'A'-'Z', 'a'-'z', '0'-'9', '-', '_', '.', ':'
#[allow(dead_code)]
pub(crate) fn qualified_name(vendor: &str, class: &str, name: &str) -> String {
    format!("{}/{}={}", vendor, class, name)
}

// IsQualifiedName tests if a device name is qualified.
#[allow(dead_code)]
pub(crate) fn is_qualified_name(name: &str) -> bool {
    match parse_qualified_name(name) {
        Ok(_) => {
            println!("{} is a qualified name", name);
            true
        }
        Err(e) => {
            println!("{} is not a qualified name, {}", name, e);
            false
        }
    }
}

// ParseQualifiedName splits a qualified name into device vendor, class,
// and name. If the device fails to parse as a qualified name, or if any
// of the split components fail to pass syntax validation, vendor and
// class are returned as empty, together with the verbatim input as the
// name and an error describing the reason for failure.
pub(crate) fn parse_qualified_name(
    device: &str,
) -> Result<(String, String, String), anyhow::Error> {
    let (vendor, class, name) = parse_device(device);
    if vendor.is_empty() {
        return Err(anyhow!("unqualified device {}, missing vendor", device));
    }
    if class.is_empty() {
        return Err(anyhow!("unqualified device {}, missing class", device));
    }
    if name.is_empty() {
        return Err(anyhow!("unqualified device {}, missing name", device));
    }
    if let Err(e) = validate_vendor_name(vendor) {
        return Err(anyhow!("invalid vendor {}: {}", device, e));
    }
    if let Err(e) = validate_class_name(class) {
        return Err(anyhow!("invalid class {}: {}", device, e));
    }
    if let Err(e) = validate_device_name(name) {
        return Err(anyhow!("invalid device {}: {}", device, e));
    }
    Ok((vendor.to_string(), class.to_string(), name.to_string()))
}

// ParseDevice tries to split a device name into vendor, class, and name.
// If this fails, for instance in the case of unqualified device names,
// ParseDevice returns an empty vendor and class together with name set
// to the verbatim input.
pub(crate) fn parse_device(device: &str) -> (&str, &str, &str) {
    if device.is_empty() || device.starts_with('/') {
        return ("", "", device);
    }

    let parts: Vec<&str> = device.split('=').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return ("", "", device);
    }

    let name = parts[1];
    let (vendor, class) = parse_qualifier(parts[0]);
    if vendor.is_empty() {
        return ("", "", device);
    }
    (vendor, class, name)
}

// ParseQualifier splits a device qualifier into vendor and class.
// The syntax for a device qualifier is
//
//	"<vendor>/<class>"
//
// If parsing fails, an empty vendor and the class set to the
// verbatim input is returned.
pub(crate) fn parse_qualifier(kind: &str) -> (&str, &str) {
    let parts: Vec<&str> = kind.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return ("", kind);
    }
    (parts[0], parts[1])
}

// ValidateVendorName checks the validity of a vendor name.
// A vendor name may contain the following ASCII characters:
//   - upper- and lowercase letters ('A'-'Z', 'a'-'z')
//   - digits ('0'-'9')
//   - underscore, dash, and dot ('_', '-', and '.')
pub(crate) fn validate_vendor_name(vendor: &str) -> Result<()> {
    if let Err(e) = validate_vendor_or_class_name(vendor) {
        return Err(anyhow!("invalid vendor. {}", e));
    }

    Ok(())
}

// ValidateClassName checks the validity of class name.
// A class name may contain the following ASCII characters:
//   - upper- and lowercase letters ('A'-'Z', 'a'-'z')
//   - digits ('0'-'9')
//   - underscore, dash, and dot ('_', '-', and '.')
pub(crate) fn validate_class_name(class: &str) -> Result<()> {
    if let Err(e) = validate_vendor_or_class_name(class) {
        return Err(anyhow!("invalid class. {}", e));
    }

    Ok(())
}

// validateVendorOrClassName checks the validity of vendor or class name.
// A name may contain the following ASCII characters:
//   - upper- and lowercase letters ('A'-'Z', 'a'-'z')
//   - digits ('0'-'9')
//   - underscore, dash, and dot ('_', '-', and '.')
pub(crate) fn validate_vendor_or_class_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("empty name"));
    }
    if !name.chars().next().unwrap_or_default().is_alphabetic() {
        return Err(anyhow!("name should start with a letter"));
    }
    if let Some(c) = name
        .chars()
        .find(|&c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.')
    {
        return Err(anyhow!("invalid character '{}' in name {}", c, name));
    }
    Ok(())
}

// ValidateDeviceName checks the validity of a device name.
// A device name may contain the following ASCII characters:
//   - upper- and lowercase letters ('A'-'Z', 'a'-'z')
//   - digits ('0'-'9')
//   - underscore, dash, dot, colon ('_', '-', '.', ':')
pub(crate) fn validate_device_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("empty name"));
    }
    if let Some(c) = name
        .chars()
        .find(|&c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':')
    {
        return Err(anyhow!("invalid character '{}' in device name {}", c, name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::parser;

    #[test]

    fn qualified_name() {
        let vendor = "nvidia.com";
        let class = "gpu";
        let name = "0";
        let device = parser::qualified_name(vendor, class, name);
        assert_eq!(device, "nvidia.com/gpu=0");
        assert!(parser::is_qualified_name(&device));
    }

    #[test]
    fn parse_qualified_name() {
        let device = "nvidia.com/gpu=0";
        match parser::parse_qualified_name(device) {
            Ok((vendor, class, name)) => {
                assert_eq!(vendor, "nvidia.com");
                assert_eq!(class, "gpu");
                assert_eq!(name, "0");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    #[test]
    fn parse_device() {
        let device = "nvidia.com/gpu=0";
        let (vendor, class, name) = parser::parse_device(device);
        assert_eq!(vendor, "nvidia.com");
        assert_eq!(class, "gpu");
        assert_eq!(name, "0");
    }

    #[test]
    fn parse_qualifier() {
        let qualifier = "nvidia.com/gpu";
        let (vendor, class) = parser::parse_qualifier(qualifier);
        assert_eq!(vendor, "nvidia.com");
        assert_eq!(class, "gpu");
    }

    #[test]
    fn validate_vendor_name() {
        let vendor = "nvidia.com";
        assert!(parser::validate_vendor_name(vendor).is_ok());

        let vendor = "nvi((dia";
        assert!(parser::validate_vendor_name(vendor).is_err());
    }
    #[test]
    fn validate_class_name() {
        let class = "gpu";
        assert!(parser::validate_class_name(class).is_ok());

        let class = "g(pu";
        assert!(parser::validate_class_name(class).is_err());
    }

    #[test]
    fn validate_device_name() {
        let name = "0";
        assert!(parser::validate_device_name(name).is_ok());

        let name = "0(";
        assert!(parser::validate_device_name(name).is_err());
    }
    #[test]
    fn validate_vendor_or_class_name() {
        let name = "nvidia.com";
        assert!(parser::validate_vendor_or_class_name(name).is_ok());

        let name = "nvi((dia.com";
        assert!(parser::validate_vendor_or_class_name(name).is_err());
    }
}
