pub mod error;
pub mod models;

#[cfg(feature = "call-engine")]
pub mod call_engine;

#[cfg(all(feature = "yaml", feature = "protobuf"))]
pub mod profile_store;

#[cfg(feature = "yaml")]
pub mod yaml_parser;

#[cfg(feature = "protobuf")]
pub mod compiler;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "test-utils")]
pub mod test_utils;

#[cfg(feature = "protobuf")]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/ycallr.rs"));
}

pub use error::{Result, YcallrError};
pub use models::*;

/// Crate version (matches `Cargo.toml`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "client")]
pub use client::{ApiError, ApiResponse, YcallrClient};
