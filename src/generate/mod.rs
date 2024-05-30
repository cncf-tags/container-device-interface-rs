/// In cdi-go, the oci-runtime-tools generate tool is used to generate configuration JSON for the OCI runtime.
/// However, since it is not possible to use existing libraries like cdi-go, we had to implement this functionality
/// ourselves. Taking this opportunity, we hope to make this version the starting point for expanding oci-runtime-tools-rs
/// and providing better support for the OCI runtime.
/// It is important to note that at this stage, our primary focus is on implementations related to our cdi-rs project.
pub mod config;
pub mod generator;
