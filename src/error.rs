use std::panic::Location;

#[derive(Debug)]
pub enum BabataError {
    Config(String, &'static Location<'static>),
    Provider(String, &'static Location<'static>),
    Memory(String, &'static Location<'static>),
    Tool(String, &'static Location<'static>),
    Channel(String, &'static Location<'static>),
    Internal(String, &'static Location<'static>),
}

impl BabataError {
    #[track_caller]
    pub fn config(message: impl Into<String>) -> Self {
        BabataError::Config(message.into(), Location::caller())
    }

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

    #[track_caller]
    pub fn channel(message: impl Into<String>) -> Self {
        BabataError::Channel(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn internal(message: impl Into<String>) -> Self {
        BabataError::Internal(message.into(), Location::caller())
    }
}

impl std::fmt::Display for BabataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BabataError::Config(msg, loc) => write!(f, "Config error at {}: {}", loc, msg),
            BabataError::Provider(msg, loc) => write!(f, "Provider error at {}: {}", loc, msg),
            BabataError::Memory(msg, loc) => write!(f, "Memory error at {}: {}", loc, msg),
            BabataError::Tool(msg, loc) => write!(f, "Tool error at {}: {}", loc, msg),
            BabataError::Channel(msg, loc) => write!(f, "Channel error at {}: {}", loc, msg),
            BabataError::Internal(msg, loc) => write!(f, "Internal error at {}: {}", loc, msg),
        }
    }
}

impl std::error::Error for BabataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<serde_json::Error> for BabataError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        BabataError::Provider(err.to_string(), Location::caller())
    }
}

impl From<std::io::Error> for BabataError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        BabataError::Internal(err.to_string(), Location::caller())
    }
}
