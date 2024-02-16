use anyhow::{Result, anyhow};

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
pub(crate) fn qualified_name(vendor: &str, class: &str, name: &str) -> String {
    format!("{}/{}={}", vendor, class, name)
}
// IsQualifiedName tests if a device name is qualified.
pub(crate) fn is_qualified_name(name: &str) -> bool {
    match parse_qualified_name(name) {
        Ok(_) => {
            print!("{} is a qualified name\n", name);
            true
        },
        Err(e) => {
            println!("{} is not a qualified name, {}\n", name, e);
            false
        },

    }
}
// ParseQualifiedName splits a qualified name into device vendor, class,
// and name. If the device fails to parse as a qualified name, or if any
// of the split components fail to pass syntax validation, vendor and
// class are returned as empty, together with the verbatim input as the
// name and an error describing the reason for failure.
pub(crate) fn parse_qualified_name(device: &str) -> Result<(String, String, String), anyhow::Error> {
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
    match validate_vendor_name(vendor) {
        Err(e) => return Err(anyhow!("invalid device {}: {}", device, e)),
        _ => (),
    }
    match validate_class_name(class) {
        Err(e) => return Err(anyhow!("invalid device {}: {}", device, e)),
        _ => (),
    }
    match validate_device_name(name) {
        Err(e) => return Err(anyhow!("invalid device {}: {}", device, e)),
        _ => (),
    }
    Ok((vendor.to_string(), class.to_string(), name.to_string()))
}
// ParseDevice tries to split a device name into vendor, class, and name.
// If this fails, for instance in the case of unqualified device names,
// ParseDevice returns an empty vendor and class together with name set
// to the verbatim input.
pub(crate) fn parse_device(device: &str) -> (&str, &str, &str) {
    if device.is_empty() || device.chars().next().unwrap() == '/' {
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
    match validate_vendor_or_class_name(vendor) {
        Err(e) => Err(anyhow!("invalid vendor. {}", e)),
        _ => Ok(()),
    }
}
// ValidateClassName checks the validity of class name.
// A class name may contain the following ASCII characters:
//   - upper- and lowercase letters ('A'-'Z', 'a'-'z')
//   - digits ('0'-'9')
//   - underscore, dash, and dot ('_', '-', and '.')
pub(crate) fn validate_class_name(class: &str) -> Result<()> {
    match validate_vendor_or_class_name(class) {
        Err(e) => Err(anyhow!("invalid class. {}", e)),
        _ => Ok(()),
    }
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
    if let Some(c) = name.chars().find(|&c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.') {
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
	if let Some(c) = name.chars().find(|&c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':') {
	    return Err(anyhow!("invalid character '{}' in device name {}", c, name));
	}
	Ok(())
    }