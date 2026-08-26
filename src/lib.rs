pub mod models;
pub mod error;

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

pub use models::*;
pub use error::{YcallrError, Result};

#[cfg(feature = "client")]
pub use client::{YcallrClient, ApiResponse, ApiError, AuthConfig};
