#[derive(Debug)]
pub enum AIError {
    NetworkError(String),
    ParseError(String),
    ValidationError(String),
    AuthenticationError(String),
    RateLimitError(String),
    APIError(String),
    ResponseParseError(String),
    UnknownError(String),
}

impl std::fmt::Display for AIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse Error: Could not understand AI response - {}. Please try rephrasing your request.", msg),
            Self::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            Self::AuthenticationError(msg) => write!(f, "Authentication Error: {}. Please check your API key in the config file.", msg),
            Self::RateLimitError(msg) => write!(f, "Rate Limit Error: {}. Please wait a moment and try again.", msg),
            Self::APIError(msg) => write!(f, "API Error: {}. Please try again or check your request.", msg),
            Self::ResponseParseError(msg) => write!(f, "Response parse error: {}", msg),
            Self::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for AIError {}
