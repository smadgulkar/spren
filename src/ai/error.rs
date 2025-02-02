#[derive(Debug)]
pub enum AIError {
    NetworkError(String),
    ParseError(String),
    ValidationError(String),
    AuthenticationError(String),
    RateLimitError(String),
    APIError(String),
}

impl std::fmt::Display for AIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            Self::ParseError(_) => write!(f, "Failed to parse AI response. Please try again."),
            Self::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            Self::AuthenticationError(msg) => write!(f, "Authentication Error: {}", msg),
            Self::RateLimitError(msg) => write!(f, "Rate Limit Error: {}", msg),
            Self::APIError(msg) => write!(f, "API Error: {}", msg),
        }
    }
}

impl std::error::Error for AIError {} 