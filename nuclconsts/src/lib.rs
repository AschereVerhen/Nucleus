use nix::unistd;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

pub mod paths;

pub static FIRST_RUN: RwLock<bool> = RwLock::new(true);

pub static IS_ROOT: LazyLock<bool> = LazyLock::new(|| unistd::Uid::effective().is_root());

pub static ALREADY_RUNNING: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
