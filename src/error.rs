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

    #[error("Environment variable error: {0}")]
    EnvVar(String),
}

pub type Result<T> = std::result::Result<T, YcallrError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = YcallrError::HttpClient("connection refused".to_string());
        assert_eq!(err.to_string(), "HTTP client error: connection refused");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: YcallrError = io_err.into();
        assert!(matches!(err, YcallrError::Io(_)));
    }
}
