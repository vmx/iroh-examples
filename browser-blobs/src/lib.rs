#[cfg(not(target_family = "wasm"))]
mod node;
#[cfg(not(target_family = "wasm"))]
pub use node::BlobsNode;

#[cfg(target_family = "wasm")]
mod wasm;

mod hasher;
