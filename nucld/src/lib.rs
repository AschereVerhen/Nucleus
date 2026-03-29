pub mod autostart;
pub mod exec;
pub mod prelude;
pub mod units;

use crate::prelude::*;
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

pub fn get_unit_from_name(name: &String) -> Option<SharedUnit> {
    UNITS.get(name).cloned()
}

pub fn get_units() -> Vec<SharedUnit> {
    UNITS.values().cloned().collect()
}
