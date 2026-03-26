use crate::errors::NuclErrors;
use crate::units::Unit;
use nix::unistd;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, RwLock};

pub mod autostart;
pub mod errors;
pub mod exec;
pub mod log;
pub mod units;

pub static FIRST_RUN: RwLock<bool> = RwLock::new(true);

pub static IS_ROOT: LazyLock<bool> = LazyLock::new(|| unistd::Uid::effective().is_root());

static NUCLD_HELPER_BINARIES: LazyLock<HashMap<String, PathBuf>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    println!("Hashmap creation started.");
    const BINARIES: &[&str] = &["nucld", "nuclctl", "nuclstart"];
    for bin in BINARIES {
        if let Ok(path) = which::which(bin) {
            map.insert(bin.to_string(), path);
        }
    }

    println!("{map:?}");
    map
});

pub fn get_path_of(name: &String) -> Result<PathBuf, NuclErrors> {
    if let Some(path) = NUCLD_HELPER_BINARIES.get(name) {
        println!("Found path: {}", path.display());
        Ok(path.clone())
    } else {
        println!("Was not able to find the path");
        Err(NuclErrors::BinaryNotFound { name: name.clone() })
    }
}

static ALREADY_RUNNING: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn mark_name_as_running(name: String, child_id: u32) -> Result<(), NuclErrors> {
    let mut w = ALREADY_RUNNING.write()?;
    (*w).insert(name, child_id);
    Ok(())
}

pub fn unmark_name_as_running(name: &String) -> Result<(), NuclErrors> {
    let mut w = ALREADY_RUNNING.write()?;
    (*w).remove(name);
    Ok(())
}

pub fn query_if_name_is_running(name: &String) -> Result<bool, NuclErrors> {
    let w = ALREADY_RUNNING.read()?;
    Ok((*w).contains_key(name))
}

pub fn get_pid_of(name: &String) -> Result<u32, NuclErrors> {
    if !query_if_name_is_running(name)? {
        return Err(NuclErrors::UnitNotRunning { name: name.clone() });
    }
    let w = &*ALREADY_RUNNING.read()?;
    unsafe { Ok(*w.get(name).unwrap_unchecked()) }
}

//Faster evals.
static UNITS: LazyLock<HashMap<String, Arc<Unit>>> = LazyLock::new(|| {
    let vec = crate::units::read_and_eval_units().expect("Failed to eval units");
    let mut hashmap = HashMap::new();
    for unit in vec.into_iter() {
        hashmap.insert(unit.get_name().clone(), Arc::new(unit));
    }
    hashmap
});

pub fn get_unit_from_name(name: &String) -> Option<Arc<Unit>> {
    UNITS.get(name).cloned()
}

pub fn get_units() -> Vec<Arc<Unit>> {
    UNITS.values().cloned().collect()
}
