use core::fmt;

#[derive(Debug)]
pub enum CoreError {
    StreamError(String),
    PersistenceError(String),
    NotificationError(String),
    AlreadyExistsError(String),
}

impl CoreError {
    pub fn message(&self) -> &str {
        match self {
            CoreError::StreamError(msg) => msg,
            CoreError::PersistenceError(msg) => msg,
            CoreError::NotificationError(msg) => msg,
            CoreError::AlreadyExistsError(msg) => msg,
        }
    }
}

// Implement display and std::error so that CoreError can be cast into the discord type error
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::StreamError(msg) => write!(f, "Stream error: {msg}"),
            CoreError::PersistenceError(msg) => write!(f, "Persistence error: {msg}"),
            CoreError::NotificationError(msg) => write!(f, "Notification error: {msg}"),
            CoreError::AlreadyExistsError(msg) => write!(f, "Already exists error: {msg}"),
        }
    }
}

impl std::error::Error for CoreError {}
