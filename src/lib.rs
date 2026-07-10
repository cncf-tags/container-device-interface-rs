pub mod annotations;
pub mod cache;
pub mod container_edits;
pub mod container_edits_unix;
pub mod default_cache;
pub mod device;
pub mod generate;
pub mod internal;
pub mod parser;
pub mod resolved_edits;
pub mod schema;
pub mod spec;
pub mod spec_dirs;
pub mod specs;
pub mod utils;
pub mod version;

pub use resolved_edits::{
    CdiEditScope, ResolvedCdiDeviceNode, ResolvedCdiEdits, ResolvedCdiMount, UnsupportedCdiEdit,
    UnsupportedCdiEditKind,
};

#[cfg(test)]
mod tests {}
