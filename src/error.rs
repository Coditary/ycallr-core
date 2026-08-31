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

    #[error("{message}")]
    ProfileNotInstalled { name: String, message: String },

    #[error("Profile install error: {0}")]
    ProfileInstall(String),
}

pub type Result<T> = std::result::Result<T, YcallrError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = YcallrError::HttpClient("connection refused".to_string());
        assert_eq!(err.to_string(), "HTTP client error: connection refused");
        assert!(YcallrError::YamlParse("bad".into())
            .to_string()
            .contains("YAML"));
        assert!(YcallrError::InvalidDefinition("bad".into())
            .to_string()
            .contains("Invalid"));
        assert!(YcallrError::CommandNotFound("x".into())
            .to_string()
            .contains("not found"));
        assert!(YcallrError::ParamValidation("bad".into())
            .to_string()
            .contains("Parameter"));
        assert!(YcallrError::Protobuf("bad".into())
            .to_string()
            .contains("Protobuf"));
        assert!(YcallrError::Serialization("bad".into())
            .to_string()
            .contains("Serialization"));
        assert!(YcallrError::EnvVar("bad".into())
            .to_string()
            .contains("Environment"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: YcallrError = io_err.into();
        assert!(matches!(err, YcallrError::Io(_)));
    }
}
