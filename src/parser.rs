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

/// Returns whether `name` is a syntactically valid CDI qualified device name.
///
/// The syntax is `<vendor>/<class>=<device>`. Vendor and class names must start
/// with an ASCII letter and end with an ASCII letter or digit. Device names
/// must start and end with an ASCII letter or digit. Interior characters may
/// be ASCII letters, digits, `_`, `-`, or `.`; device names also allow `:`.
/// Single ASCII letters are valid components, and single digits are valid
/// device names.
///
/// This function checks syntax only, without checking whether the device
/// exists. The input is neither trimmed nor normalized.
/// Host paths, malformed names, non-ASCII characters, and comma-separated
/// lists are rejected.
/// Use [`parse_qualified_name`] to obtain the components or a validation error.
///
/// ```
/// use container_device_interface::parser::is_qualified_name;
///
/// assert!(is_qualified_name("example.com/accelerator=device0"));
/// assert!(is_qualified_name("nvidia.com/gpu=0:1"));
/// assert!(!is_qualified_name("/dev/nvidia0"));
/// assert!(!is_qualified_name("example.com/accelerator="));
/// ```
pub fn is_qualified_name(name: &str) -> bool {
    parse_qualified_name(name).is_ok()
}

/// Parses a CDI qualified device name into borrowed `(vendor, class, name)` slices.
///
/// The input must satisfy the ASCII grammar documented by [`is_qualified_name`].
/// Components borrow from the input and are returned verbatim, without trimming
/// or normalization. Successful parsing allocates no strings. This function
/// checks syntax only; it does not check whether the device exists.
///
/// # Errors
///
/// Returns an error describing a missing or invalid component if the input is
/// not a valid qualified name. No components are returned on failure. Host
/// paths, extra `/` or `=` separators, whitespace, non-ASCII characters, and
/// comma-separated lists are rejected.
///
/// ```
/// use container_device_interface::parser::parse_qualified_name;
///
/// let (vendor, class, name) = parse_qualified_name("example.com/accelerator=card0")?;
/// assert_eq!((vendor, class, name),
///            ("example.com", "accelerator", "card0"));
/// let (_, _, mig_name) = parse_qualified_name("nvidia.com/gpu=0:1")?;
/// assert_eq!(mig_name, "0:1");
/// assert!(parse_qualified_name("/dev/nvidia0").is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_qualified_name(device: &str) -> Result<(&str, &str, &str)> {
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
        return Err(anyhow!("invalid device {}: {}", device, e));
    }
    if let Err(e) = validate_class_name(class) {
        return Err(anyhow!("invalid device {}: {}", device, e));
    }
    if let Err(e) = validate_device_name(name) {
        return Err(anyhow!("invalid device {}: {}", device, e));
    }
    Ok((vendor, class, name))
}

// ParseDevice tries to split a device name into vendor, class, and name.
// If this fails, for instance in the case of unqualified device names,
// ParseDevice returns an empty vendor and class together with name set
// to the verbatim input.
// Empty components and extra separators are left for syntax validation.
pub(crate) fn parse_device(device: &str) -> (&str, &str, &str) {
    if device.is_empty() || device.starts_with('/') {
        return ("", "", device);
    }

    let Some((qualifier, name)) = device.split_once('=') else {
        return ("", "", device);
    };

    let (vendor, class) = parse_qualifier(qualifier);
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
// Empty components and extra separators are left for syntax validation.
pub(crate) fn parse_qualifier(kind: &str) -> (&str, &str) {
    kind.split_once('/').unwrap_or(("", kind))
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
    if !name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return Err(anyhow!("name should start with a letter"));
    }
    if !name
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return Err(anyhow!("name should end with a letter or digit"));
    }
    let char_count = name.chars().count();
    if let Some(c) = name
        .chars()
        .skip(1)
        .take(char_count.saturating_sub(2))
        .find(|&c| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.')
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
    if !name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return Err(anyhow!("name should start with a letter or digit"));
    }
    if !name
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return Err(anyhow!("name should end with a letter or digit"));
    }
    let char_count = name.chars().count();
    if let Some(c) = name
        .chars()
        .skip(1)
        .take(char_count.saturating_sub(2))
        .find(|&c| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':')
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
        assert_eq!(
            parser::parse_qualified_name(device).unwrap(),
            ("nvidia.com", "gpu", "0")
        );
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

    #[test]
    fn validate_names_require_alphanumeric_endpoints() {
        assert!(parser::validate_vendor_name("vendor.com").is_ok());
        assert!(parser::validate_vendor_name("vendor.").is_err());
        assert!(parser::validate_class_name("gpu-").is_err());
        assert!(parser::validate_device_name("_gpu").is_err());
        assert!(parser::validate_device_name("gpu_").is_err());
    }

    #[test]
    fn qualified_name_detection_both_ways() {
        assert!(parser::is_qualified_name("vendor.com/class=dev"));
        assert!(!parser::is_qualified_name("/dev/null"));
    }

    #[test]
    fn parse_qualified_name_rejects_malformed_input() {
        for (input, needle) in [
            ("", "missing vendor"),
            ("/dev/null", "missing vendor"),
            ("class=dev", "missing vendor"),
            ("vendor.com/class", "missing vendor"),
            ("vendor.com/class=", "missing name"),
            ("vendor.com/=dev", "missing class"),
            ("vendor.com/class=device=other", "invalid character '='"),
            ("vendor.com/class/extra=dev", "invalid character '/'"),
            ("_vendor/class=dev", "invalid vendor"),
            ("vendor.com/cl*ss=dev", "invalid class"),
            ("vendor.com/class=dev name", "invalid device"),
        ] {
            let err = parser::parse_qualified_name(input).unwrap_err();
            assert!(
                err.to_string().contains(needle),
                "{input:?}: expected {needle:?} in {err}"
            );
        }
    }

    #[test]
    fn parse_qualifier_splits_or_returns_verbatim() {
        assert_eq!(parser::parse_qualifier("vendor/class"), ("vendor", "class"));
        assert_eq!(parser::parse_qualifier("noslash"), ("", "noslash"));
        assert_eq!(parser::parse_qualifier("vendor/"), ("vendor", ""));
    }

    #[test]
    fn parsing_errors_identify_components_without_duplicate_prefixes() {
        for (input, expected) in [
            ("vendor.com/class=", "unqualified device vendor.com/class=, missing name"),
            ("vendor.com/=dev", "unqualified device vendor.com/=dev, missing class"),
            ("_v/class=dev", "invalid device _v/class=dev: invalid vendor. name should start with a letter"),
            ("v/_class=dev", "invalid device v/_class=dev: invalid class. name should start with a letter"),
            ("v/class=device=other", "invalid device v/class=device=other: invalid character '=' in device name device=other"),
        ] {
            assert_eq!(parser::parse_qualified_name(input).unwrap_err().to_string(), expected);
        }
    }

    #[test]
    fn accepts_qualified_names_for_any_vendor() {
        for (vendor, class, name) in [
            ("example.com", "accelerator", "card0"),
            ("intel.com", "gpu", "renderD128"),
            ("amd.com", "gpu", "all"),
            ("Vendor_1.example", "Class-v2.0", "Dev_0-1.2:3"),
            ("A", "z", "9"),
            ("vendor", "class", "a::b"),
            ("nvidia.com", "gpu", "0"),
            (
                "nvidia.com",
                "gpu",
                "GPU-12345678-abcd-1234-abcd-123456789abc",
            ),
            ("nvidia.com", "gpu", "0:1"),
            (
                "nvidia.com",
                "gpu",
                "MIG-12345678-abcd-1234-abcd-123456789abc",
            ),
            ("nvidia.com", "gpu", "all"),
        ] {
            let selector = format!("{vendor}/{class}={name}");
            assert_eq!(
                parser::parse_qualified_name(&selector).unwrap(),
                (vendor, class, name),
                "{selector:?}"
            );
            assert!(parser::is_qualified_name(&selector), "{selector:?}");
        }
    }

    #[test]
    fn rejects_malformed_names_and_host_paths() {
        for selector in [
            "",
            "0",
            "all",
            "vendor.com/class",
            "class=device",
            "/class=device",
            "vendor.com/=device",
            "vendor.com/class=",
            "vendor.com/class=device=other",
            "vendor.com/class/extra=device",
            "vendor.com/class=device/extra",
            "vendor.com/class=dev,other",
            "vendor.com/class=0,vendor.com/class=1",
            "/dev/nvidia0",
            "/dev/dri/renderD128",
            "./dev/nvidia0",
            "../dev/nvidia0",
            "/vendor.com/class=device",
            " vendor.com/class=device",
            "vendor.com/class=device ",
            "vendor.com/class=dev\tice",
            "vendor.com/class=device\n",
            "vendor.com/class=dev\0ice",
            "vendor.com/class=*",
        ] {
            assert!(
                parser::parse_qualified_name(selector).is_err(),
                "{selector:?}"
            );
            assert!(!parser::is_qualified_name(selector), "{selector:?}");
        }
    }

    #[test]
    fn enforces_ascii_component_grammar() {
        for name in ["A", "z", "Vendor", "a0", "a_Z-9.x"] {
            assert!(
                parser::is_qualified_name(&format!("{name}/class=dev")),
                "{name:?}"
            );
            assert!(
                parser::is_qualified_name(&format!("vendor/{name}=dev")),
                "{name:?}"
            );
        }
        for name in [
            "", "0", "9vendor", "_a", "-a", ".a", "a_", "a-", "a.", "a:b", "a/b", "a=b", "a b",
            "@a", "[a", "`a", "{a", "a@b", "a[b", "a`b", "a{b", "é", "éa", "aé", "aéb", "中",
            "a中b", "a😀b",
        ] {
            for selector in [format!("{name}/class=dev"), format!("vendor/{name}=dev")] {
                assert!(
                    parser::parse_qualified_name(&selector).is_err(),
                    "{selector:?}"
                );
                assert!(!parser::is_qualified_name(&selector), "{selector:?}");
            }
        }

        for name in ["A", "z", "0", "9", "a_Z-9.x:0", "0:1", "a::b"] {
            assert!(
                parser::is_qualified_name(&format!("vendor/class={name}")),
                "{name:?}"
            );
        }
        for name in [
            "",
            "_a",
            "-a",
            ".a",
            ":a",
            "a_",
            "a-",
            "a.",
            "a:",
            "/dev/null",
            "a/b",
            "a=b",
            "a,b",
            "a b",
            "/",
            ":",
            "@a",
            "[a",
            "`a",
            "{a",
            "a@b",
            "a[b",
            "a`b",
            "a{b",
            "é",
            "éa",
            "aé",
            "aéb",
            "中",
            "a中b",
            "a😀b",
        ] {
            let selector = format!("vendor/class={name}");
            assert!(
                parser::parse_qualified_name(&selector).is_err(),
                "{selector:?}"
            );
            assert!(!parser::is_qualified_name(&selector), "{selector:?}");
        }
    }
}
