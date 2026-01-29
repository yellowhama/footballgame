use std::fmt;

#[derive(Debug)]
pub enum MatchError {
    InvalidFormation(String),
    InvalidTeamSize { expected: usize, found: usize },
    InvalidPosition(String),
    ValidationError(String),
    SerializationError(String),
    DeserializationError(String),
}

#[derive(Debug)]
pub enum CoreError {
    InvalidParameter(String),
    NotFound(String),
    NotInitialized(String),
    ProcessingError(String),
    SerializationError(String),
    DeserializationError(String),
    IoError(String),
    ParseError(String),
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MatchError::InvalidFormation(formation) => {
                write!(f, "Invalid formation: {}", formation)
            }
            MatchError::InvalidTeamSize { expected, found } => {
                write!(f, "Invalid team size: expected {}, found {}", expected, found)
            }
            MatchError::InvalidPosition(position) => {
                write!(f, "Invalid player position: {}", position)
            }
            MatchError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
            MatchError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            MatchError::DeserializationError(msg) => {
                write!(f, "Deserialization error: {}", msg)
            }
        }
    }
}

impl std::error::Error for MatchError {}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CoreError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            CoreError::NotFound(msg) => write!(f, "Not found: {}", msg),
            CoreError::NotInitialized(msg) => write!(f, "Not initialized: {}", msg),
            CoreError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
            CoreError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            CoreError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            CoreError::IoError(msg) => write!(f, "IO error: {}", msg),
            CoreError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<serde_json::Error> for MatchError {
    fn from(err: serde_json::Error) -> Self {
        if err.is_data() {
            MatchError::DeserializationError(err.to_string())
        } else {
            MatchError::SerializationError(err.to_string())
        }
    }
}

pub type Result<T> = std::result::Result<T, MatchError>;
