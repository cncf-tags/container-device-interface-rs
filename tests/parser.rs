// These tests prove the qualified-name APIs are usable by an external consumer.
// Exhaustive grammar and error coverage lives in src/parser.rs.

use container_device_interface::parser::{is_qualified_name, parse_qualified_name};

#[test]
fn qualified_name_apis_are_public() {
    let selector = String::from("example.com/accelerator=card0");
    let components: (&str, &str, &str) = parse_qualified_name(&selector).unwrap();
    assert_eq!(components, ("example.com", "accelerator", "card0"));
    assert!(is_qualified_name(&selector));

    assert_eq!(
        parse_qualified_name("nvidia.com/gpu=0:1").unwrap(),
        ("nvidia.com", "gpu", "0:1")
    );
    assert!(!is_qualified_name("/dev/nvidia0"));
    assert!(parse_qualified_name("/dev/nvidia0").is_err());
}
