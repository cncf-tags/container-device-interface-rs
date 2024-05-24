use anyhow::Result;
use oci_spec::runtime as oci;

use crate::{specs::config::ContainerEdits as CDIContainerEdits, utils::merge};

// ContainerEdits represent updates to be applied to an OCI Spec.
// These updates can be specific to a CDI device, or they can be
// specific to a CDI Spec. In the former case these edits should
// be applied to all OCI Specs where the corresponding CDI device
// is injected. In the latter case, these edits should be applied
// to all OCI Specs where at least one devices from the CDI Spec
// is injected.
#[derive(Clone, Default, Debug, Eq, PartialEq, Hash)]
pub struct ContainerEdits {
    pub container_edits: CDIContainerEdits,
}

impl ContainerEdits {
    // Apply edits to the given OCI Spec. Updates the OCI Spec in place.
    // Returns an error if the update fails.
    pub fn new() -> Self {
        Self {
            container_edits: CDIContainerEdits {
                ..Default::default()
            },
        }
    }

    pub fn apply(&mut self, _oci_spec: &mut oci::Spec) -> Result<()> {
        // TODO: it depends on Generator related to oci spec, however, there's no existing
        // Generator for us and the Generator depends on the MutGetters attribute on oci-spec-rs/runtime.
        // It will be implemented once the PR https://github.com/containers/oci-spec-rs/pull/166 merged
        Ok(())
    }

    pub fn append(&mut self, o: ContainerEdits) -> Self {
        let intel_rdt = if o.container_edits.intel_rdt.is_some() {
            o.container_edits.intel_rdt
        } else {
            None
        };

        let ce = CDIContainerEdits {
            env: merge(&mut self.container_edits.env, &o.container_edits.env),
            device_nodes: merge(
                &mut self.container_edits.device_nodes,
                &o.container_edits.device_nodes,
            ),
            hooks: merge(&mut self.container_edits.hooks, &o.container_edits.hooks),
            mounts: merge(&mut self.container_edits.mounts, &o.container_edits.mounts),
            intel_rdt,
            additional_gids: merge(
                &mut self.container_edits.additional_gids,
                &o.container_edits.additional_gids,
            ),
        };

        Self {
            container_edits: ce,
        }
    }
}
