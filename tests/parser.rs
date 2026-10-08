use container_device_interface::parser::{is_qualified_name, parse_qualified_name};

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
            parse_qualified_name(&selector).unwrap(),
            (vendor.to_owned(), class.to_owned(), name.to_owned()),
            "{selector:?}"
        );
        assert!(is_qualified_name(&selector), "{selector:?}");
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
        assert!(parse_qualified_name(selector).is_err(), "{selector:?}");
        assert!(!is_qualified_name(selector), "{selector:?}");
    }
}

#[test]
fn enforces_ascii_component_grammar() {
    for name in ["A", "z", "Vendor", "a0", "a_Z-9.x"] {
        assert!(is_qualified_name(&format!("{name}/class=dev")), "{name:?}");
        assert!(is_qualified_name(&format!("vendor/{name}=dev")), "{name:?}");
    }
    for name in [
        "", "0", "9vendor", "_a", "-a", ".a", "a_", "a-", "a.", "a:b", "a/b", "a=b", "a b", "@a",
        "[a", "`a", "{a", "a@b", "a[b", "a`b", "a{b", "é", "éa", "aé", "aéb", "中", "a中b", "a😀b",
    ] {
        for selector in [format!("{name}/class=dev"), format!("vendor/{name}=dev")] {
            assert!(parse_qualified_name(&selector).is_err(), "{selector:?}");
            assert!(!is_qualified_name(&selector), "{selector:?}");
        }
    }

    for name in ["A", "z", "0", "9", "a_Z-9.x:0", "0:1", "a::b"] {
        assert!(
            is_qualified_name(&format!("vendor/class={name}")),
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
        assert!(parse_qualified_name(&selector).is_err(), "{selector:?}");
        assert!(!is_qualified_name(&selector), "{selector:?}");
    }
}

#[test]
fn parsing_errors_describe_invalid_components() {
    for (selector, reason) in [
        ("/dev/null", "missing vendor"),
        ("_vendor/class=dev", "invalid vendor"),
        ("vendor/cl*ss=dev", "invalid class"),
        ("vendor/class=dev name", "invalid device"),
    ] {
        let error = parse_qualified_name(selector).unwrap_err().to_string();
        assert!(error.contains(reason), "{selector:?}: {error}");
    }
}
