use std::fmt;
use serde_json::Error as JsonError;

#[derive(Debug)]
pub enum AIError {
    NetworkError(String),
    ParseError(String),
    ValidationError(String),
    AuthenticationError(String),
    RateLimitError(String),
    APIError(String),
}

impl fmt::Display for AIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            Self::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
            Self::APIError(msg) => write!(f, "API error: {}", msg),
        }
    }
}

impl std::error::Error for AIError {}

impl From<JsonError> for AIError {
    fn from(error: JsonError) -> Self {
        AIError::ParseError(format!("JSON serialization error: {}", error))
    }
} 