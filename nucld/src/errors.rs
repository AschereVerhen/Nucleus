use std::any::Any;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NuclErrors {
    #[error("The unit {name} is not running.")]
    UnitNotRunning { name: String },
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Thread Error: {0}")]
    ThreadPanic(String),
    #[error(
        "In the file {filename}, the name of the file does not match the name of the unit in the file."
    )]
    NameMismatch { filename: String },
    #[error("Toml Parsing Error: {0}")]
    TomlParsingError(#[from] toml::de::Error),
    #[error("Json Parsing Error: {0}")]
    JsonParsingError(#[from] serde_json::Error),
    #[error("Unix Syscall failed: {0}")]
    UnixSyscallFailure(#[from] nix::Error),
    #[error("The following binary was not found on the system: {name}")]
    BinaryNotFound { name: String },
    #[error("Failed to get a lock on a variable")]
    FailedToGetRwLock(String),
}

impl<T> From<std::sync::PoisonError<T>> for NuclErrors {
    fn from(value: std::sync::PoisonError<T>) -> Self {
        NuclErrors::FailedToGetRwLock(value.to_string())
    }
}

pub fn extract_panic_message(panic: Box<dyn Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

#[macro_export]
macro_rules! thread {
    ($func: expr $(, $name_for_thread: expr)? $(,)?) => {{
        std::thread::Builder::new()
            $(.name($name_for_thread))?
            .spawn($func)
            .map_err(|e| $crate::errors::NuclErrors::IO(e))
    }}
}
