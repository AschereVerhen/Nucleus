#![allow(unused_imports)]

// ===== External crates =====
pub use nix::unistd::Uid;

// ===== Your crates =====
pub use nuclconsts::paths::*;
pub use nuclconsts::*;

pub use nuclerrors::{NuclErrors, NuclResult, extract_panic_message};
pub use nucllib::commands::Commands;

// Macro export (FIXES your `thread!` error)
pub use nucllib::thread;

// ===== Std =====
pub use std::collections::{HashMap, HashSet};
pub use std::path::PathBuf;
pub use std::process::{self, Command};
pub use std::sync::{Arc, LazyLock, Mutex, RwLock};

// Filesystem / traversal (FIXES DirEntry + WalkDir errors)
pub use std::fs::DirEntry;
pub use walkdir::WalkDir;

// ===== Crate items =====
pub use crate::exec::Exec;

pub use nuclconsts::units::{RunningRegistry, SharedUnit, Unit, UnitFS, UnitRegistry};

// ===== Traits =====
pub use serde::{Deserialize, Serialize};
pub use std::io::{Read, Write};
pub use tabled::Tabled;

// ==== Tracing =====
pub use tracing::{
    debug, debug_span, error, error_span, info, info_span, instrument, trace, trace_span,
};
