mod context;
mod prepare;
pub mod templates;
mod types;

pub use context::{resolve_client_env, ClientContext};
#[cfg(not(target_arch = "wasm32"))]
pub use prepare::NativeMultipartPart;
pub use prepare::{build_api_response, prepare_http_request, PreparedBody, PreparedHttpRequest};
pub use types::{ApiError, ApiResponse, AuthConfig, EnvMode};
