// Integration tests link the crate like an external consumer, so this is
// what proves the spec structs are publicly constructible — the unit tests
// in src/ compiled even when the fields were pub(crate).

use std::collections::BTreeMap;

use container_device_interface::spec::validate_spec;
use container_device_interface::specs::config::{
    ContainerEdits, Device, DeviceNode, Hook, IntelRdt, LinuxNetDevice, Mount, Spec,
};

fn spec_with_one_gpu() -> Spec {
    Spec {
        version: "1.1.0".to_string(),
        kind: "vendor.com/gpu".to_string(),
        annotations: BTreeMap::from([("vendor.com/key".to_string(), "value".to_string())]),
        devices: vec![Device {
            name: "gpu0".to_string(),
            annotations: BTreeMap::new(),
            container_edits: ContainerEdits {
                env: Some(vec!["GPU=0".to_string()]),
                device_nodes: Some(vec![DeviceNode {
                    path: "/dev/gpu0".to_string(),
                    host_path: Some("/dev/gpu0".to_string()),
                    r#type: Some("c".to_string()),
                    major: Some(226),
                    minor: Some(0),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        }],
        container_edits: Some(ContainerEdits {
            mounts: Some(vec![Mount {
                host_path: "/usr/lib/vendor".to_string(),
                container_path: "/usr/lib/vendor".to_string(),
                r#type: Some("bind".to_string()),
                options: Some(vec!["ro".to_string()]),
            }]),
            hooks: Some(vec![Hook {
                hook_name: "createContainer".to_string(),
                path: "/usr/bin/vendor-hook".to_string(),
                args: None,
                env: None,
                timeout: None,
            }]),
            net_devices: Some(vec![LinuxNetDevice {
                host_interface_name: "eth0".to_string(),
                name: "veth0".to_string(),
            }]),
            intel_rdt: Some(IntelRdt {
                schemata: Some(vec!["L3:0=ffff".to_string()]),
                ..Default::default()
            }),
            additional_gids: Some(vec![44]),
            ..Default::default()
        }),
    }
}

#[test]
fn spec_is_constructible_and_validates() {
    validate_spec(&spec_with_one_gpu()).expect("programmatically built spec should validate");
}

#[test]
fn spec_serializes_without_yaml_round_trip() {
    let spec = spec_with_one_gpu();

    let json = serde_json::to_string(&spec).expect("spec serializes to JSON");
    assert!(json.contains("\"cdiVersion\":\"1.1.0\""));
    assert!(json.contains("\"kind\":\"vendor.com/gpu\""));

    let parsed: Spec = serde_json::from_str(&json).expect("spec round-trips");
    assert_eq!(spec, parsed);
}
