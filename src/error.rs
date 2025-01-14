use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{driver} error: {message}")]
    Backend {
        driver: &'static str,
        message: String,
        /// Source SQL when the backend reported a location.
        sql: Option<String>,
        /// Byte offset into `sql` (start of the bad token), if known.
        offset: Option<usize>,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("readline error: {0}")]
    Readline(#[from] rustyline::error::ReadlineError),

    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn msg(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }

    pub fn backend(driver: &'static str, message: impl Into<String>) -> Self {
        Self::Backend {
            driver,
            message: message.into(),
            sql: None,
            offset: None,
        }
    }

    pub fn backend_at(
        driver: &'static str,
        message: impl Into<String>,
        sql: impl Into<String>,
        offset: usize,
    ) -> Self {
        Self::Backend {
            driver,
            message: message.into(),
            sql: Some(sql.into()),
            offset: Some(offset),
        }
    }

    pub fn driver(&self) -> Option<&'static str> {
        match self {
            Self::Backend { driver, .. } => Some(*driver),
            _ => None,
        }
    }

    pub fn message_text(&self) -> String {
        match self {
            Self::Backend { message, .. } => message.clone(),
            Self::Message(m) => m.clone(),
            other => other.to_string(),
        }
    }

    pub fn sql_snippet(&self) -> Option<&str> {
        match self {
            Self::Backend { sql: Some(s), .. } => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn offset(&self) -> Option<usize> {
        match self {
            Self::Backend { offset, .. } => *offset,
            _ => None,
        }
    }
}
