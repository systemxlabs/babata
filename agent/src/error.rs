use std::panic::Location;

#[derive(Debug)]
pub enum BabataError {
    Provider(String, &'static Location<'static>),
    Memory(String, &'static Location<'static>),
    Tool(String, &'static Location<'static>),
}

impl BabataError {
    #[track_caller]
    pub fn provider(message: impl Into<String>) -> Self {
        BabataError::Provider(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn memory(message: impl Into<String>) -> Self {
        BabataError::Memory(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn tool(message: impl Into<String>) -> Self {
        BabataError::Tool(message.into(), Location::caller())
    }
}

impl std::fmt::Display for BabataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BabataError::Provider(msg, loc) => write!(f, "Provider error at {}: {}", loc, msg),
            BabataError::Memory(msg, loc) => write!(f, "Memory error at {}: {}", loc, msg),
            BabataError::Tool(msg, loc) => write!(f, "Tool error at {}: {}", loc, msg),
        }
    }
}

impl std::error::Error for BabataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<serde_json::Error> for BabataError {
    fn from(err: serde_json::Error) -> Self {
        BabataError::Provider(err.to_string(), Location::caller())
    }
}
