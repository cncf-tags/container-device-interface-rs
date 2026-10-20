// Dedicated executable: the Spec validator is process-global and the
// destructor below re-enters its API. In the library's unit-test binary a
// concurrent new_spec() can still hold the Arc cloned by validate_spec, which
// would delay the destructor and let it clear another test's validator.
// Nothing else in this executable loads Specs.

use anyhow::{anyhow, Result};
use container_device_interface::spec::{
    clear_spec_validator, set_spec_validator, validate_spec, SpecValidator,
};
use container_device_interface::specs::config::Spec;

fn raw_spec() -> Spec {
    Spec {
        version: "1.1.0".to_string(),
        kind: "vendor.com/device".to_string(),
        ..Default::default()
    }
}

// Deadlocks if the previous validator is dropped while the write lock is
// held; the result of the nested validate_spec is irrelevant.
struct ReentrantOnDrop;

impl SpecValidator for ReentrantOnDrop {
    fn validate_spec(&self, _raw_spec: &Spec) -> Result<()> {
        Ok(())
    }
}

impl Drop for ReentrantOnDrop {
    fn drop(&mut self) {
        let _ = validate_spec(&raw_spec());
        clear_spec_validator();
    }
}

#[test]
fn previous_validator_is_dropped_outside_the_lock() {
    set_spec_validator(ReentrantOnDrop);

    // No other holder exists here, so replacing drops ReentrantOnDrop at
    // once and its destructor clears the replacement again.
    set_spec_validator(|_: &Spec| -> Result<()> { Err(anyhow!("rejects everything")) });
    validate_spec(&raw_spec())
        .expect("the destructor's clear_spec_validator removed the replacement");

    set_spec_validator(ReentrantOnDrop);
    clear_spec_validator();
    validate_spec(&raw_spec()).expect("no validator installed");
}
