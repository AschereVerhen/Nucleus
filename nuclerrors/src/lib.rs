use serde::{Deserialize, Serialize};
use std::any::Any;
use thiserror::Error;

#[derive(Debug, Error, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum NuclErrors {
    #[error("The unit {name} is not running.")]
    UnitNotRunning { name: String },
    #[error("IO error: {0}")]
    IO(String),
    #[error("Thread Error: {0}")]
    ThreadPanic(String),
    #[error(
        "In the file {filename}, the name of the file does not match the name of the unit in the file."
    )]
    NameMismatch { filename: String },
    #[error("Toml Parsing Error: {0}")]
    TomlParsingError(String),
    #[error("Json Parsing Error: {0}")]
    JsonParsingError(String),
    #[error("Unix Syscall failed: {0}")]
    UnixSyscallFailure(String),
    #[error("The following binary was not found on the system: {name}")]
    BinaryNotFound { name: String },
    #[error("Failed to get a lock on a variable")]
    FailedToGetRwLock(String),
    #[error("The unit: {name} is invalid.")]
    UnitIsInvalid { name: String },
    #[error("Unit \"{name}\" disappeared during stop operation")]
    UnitNotFound { name: String },
    #[error(
        "Tried to use a root-only feature of the Init Manager, while it is not running as root."
    )]
    INITIsNotRoot,
}

impl<T> From<std::sync::PoisonError<T>> for NuclErrors {
    fn from(value: std::sync::PoisonError<T>) -> Self {
        NuclErrors::FailedToGetRwLock(value.to_string())
    }
}

impl From<walkdir::Error> for NuclErrors {
    fn from(value: walkdir::Error) -> Self {
        NuclErrors::IO(value.to_string())
    }
}

impl From<serde_json::Error> for NuclErrors {
    fn from(value: serde_json::Error) -> Self {
        NuclErrors::JsonParsingError(value.to_string())
    }
}
impl From<nix::Error> for NuclErrors {
    fn from(value: nix::Error) -> Self {
        NuclErrors::UnixSyscallFailure(value.to_string())
    }
}
impl From<toml::ser::Error> for NuclErrors {
    fn from(value: toml::ser::Error) -> Self {
        NuclErrors::TomlParsingError(value.to_string())
    }
}
impl From<toml::de::Error> for NuclErrors {
    fn from(value: toml::de::Error) -> Self {
        NuclErrors::TomlParsingError(value.to_string())
    }
}

impl From<std::io::Error> for NuclErrors {
    fn from(value: std::io::Error) -> Self {
        NuclErrors::IO(value.to_string())
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

pub type NuclResult<T> = Result<T, NuclErrors>;
