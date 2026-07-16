use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error ({path}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("NBT error ({context}): {message}")]
    Nbt { context: String, message: String },

    #[error("invalid world: {0}")]
    InvalidWorld(String),

    #[error("unsupported target version: {0}")]
    UnsupportedTarget(String),

    #[error("output folder error: {0}")]
    Output(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub fn nbt(context: impl ToString, message: impl ToString) -> Self {
        Error::Nbt {
            context: context.to_string(),
            message: message.to_string(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Error::Io { .. } => "E10",
            Error::Nbt { .. } => "E20",
            Error::InvalidWorld(_) => "E30",
            Error::UnsupportedTarget(_) => "E40",
            Error::Output(_) => "E50",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Io { .. } => 10,
            Error::Nbt { .. } => 20,
            Error::InvalidWorld(_) => 30,
            Error::UnsupportedTarget(_) => 40,
            Error::Output(_) => 50,
        }
    }
}
