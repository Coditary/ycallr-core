#[derive(Debug, thiserror::Error)]
pub enum YcallrError {
    #[error("YAML parse error: {0}")]
    YamlParse(String),

    #[error("Invalid API definition: {0}")]
    InvalidDefinition(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Parameter validation error: {0}")]
    ParamValidation(String),

    #[error("Protobuf error: {0}")]
    Protobuf(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("HTTP client error: {0}")]
    HttpClient(String),
}

pub type Result<T> = std::result::Result<T, YcallrError>;
