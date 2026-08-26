pub mod error;
pub mod models;

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

#[cfg(feature = "client")]
pub use client::{ApiError, ApiResponse, AuthConfig, YcallrClient};
