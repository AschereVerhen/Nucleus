use crate::IS_ROOT;
use crate::is_root;
use crate::units_parsing::read_and_eval_units;
use nuclerrors::NuclErrors;
use nuclerrors::NuclResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::{LazyLock, RwLock};
use tabled::Tabled;
use walkdir::WalkDir;

#[derive(Default)]
pub struct UnitBuilder {
    name: Option<String>,
    cmd: Option<Vec<String>>,
    restart: bool,
    dependencies: Vec<String>,
    autostart: bool,
}

impl UnitBuilder {
    pub fn new() -> Self {
        UnitBuilder::default()
    }
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    pub fn cmd(mut self, cmd: Vec<String>) -> Self {
        self.cmd = Some(cmd);
        self
    }
    pub fn restart(mut self, restart: bool) -> Self {
        self.restart = restart;
        self
    }
    pub fn dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn autostart(mut self, autostart: bool) -> Self {
        self.autostart = autostart;
        self
    }

    pub fn build(self) -> Unit {
        Unit {
            name: self.name.ok_or("Name must be present").unwrap(),
            cmd: self.cmd.unwrap_or(vec![]),
            restart: self.restart,
            autostart: self.autostart,
            dependencies: Some(self.dependencies),
        }
    }
}

fn format_cmd(v: &[String]) -> String {
    v.join(" ")
}

fn format_optional_vec(opt: &Option<Vec<String>>) -> String {
    match opt {
        Some(v) if !v.is_empty() => v.join(", "),
        _ => "none".to_string(),
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Tabled, Default, Hash, PartialEq, Eq)]
pub struct Unit {
    name: String,
    #[tabled(display = "format_cmd")]
    cmd: Vec<String>,
    #[serde(default)]
    restart: bool,
    #[serde(default)]
    autostart: bool,
    #[tabled(display = "format_optional_vec")]
    dependencies: Option<Vec<String>>, //File names.
}

impl Unit {
    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn get_cmd(&self) -> &[String] {
        &self.cmd
    }
    pub fn get_restart(&self) -> bool {
        self.restart
    }
    pub fn get_dependencies(&self) -> Option<&[String]> {
        self.dependencies.as_deref()
    }
    pub fn get_autostart(&self) -> bool {
        self.autostart
    }
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_cmd(&mut self, cmd: Vec<String>) {
        self.cmd = cmd;
    }

    pub fn set_restart(&mut self, restart: bool) {
        self.restart = restart;
    }

    pub fn set_dependencies(&mut self, dependencies: Option<Vec<String>>) {
        self.dependencies = dependencies;
    }

    pub fn set_autostart(&mut self, autostart: bool) {
        self.autostart = autostart;
    }

    pub fn shared(self) -> SharedUnit {
        Arc::new(Mutex::new(self))
    }
}

pub struct UnitFS;

impl UnitFS {
    pub fn write_unit(unit_struct: SharedUnit, user_defined: bool) -> Result<(), NuclErrors> {
        let dirs = &*crate::paths::DIRUNIT;

        let dir = if user_defined {
            &dirs.user_dir
        } else {
            if !*IS_ROOT {
                return Err(NuclErrors::INITIsNotRoot);
            }

            &dirs.system_dir
        };
        let unit_guard = unit_struct.lock()?;

        let new_unit_file = dir.join(format!("{}.toml", unit_guard.get_name()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(new_unit_file)?;
        let serialized = toml::to_string_pretty(&*unit_guard)?;
        writeln!(&mut file, "{serialized}")?;
        Ok(())
    }

    pub fn remove_unit(unit_name: String) -> Result<(), NuclErrors> {
        println!("Remove_Unit called");
        let dirs = &*crate::paths::DIRUNIT;
        let target_name = std::ffi::OsString::from(format!("{}.toml", unit_name));
        let mut removed: bool = false;
        for entry in WalkDir::new(dirs.user_dir.as_path()) {
            let entry = entry?;
            if entry.file_name() == target_name {
                std::fs::remove_file(entry.path())?;
                removed = true;
            }
        }
        if is_root() {
            for entry in WalkDir::new(dirs.system_dir.as_path()) {
                let entry = entry?;
                if entry.file_name() == target_name {
                    std::fs::remove_file(entry.path())?;
                    removed = true;
                }
            }
        }

        if !removed {
            return Err(NuclErrors::UnitNotFound { name: unit_name });
        }

        Ok(())
    }
}
pub type SharedUnit = Arc<Mutex<Unit>>;
static UNITS_REGISTRY: LazyLock<RwLock<HashMap<String, SharedUnit>>> = LazyLock::new(|| {
    RwLock::new({
        let vec: Vec<Unit> = read_and_eval_units().expect("Failed to eval units"); //this should
        //panic if failed.
        let mut hashmap = HashMap::new();
        for unit in vec.into_iter() {
            hashmap.insert(unit.get_name().clone(), Arc::new(Mutex::new(unit)));
        }
        hashmap
    })
});

pub struct UnitRegistry;

impl UnitRegistry {
    pub fn get_unit(name: &String) -> Option<SharedUnit> {
        let guard = UNITS_REGISTRY.read().ok()?;
        guard.get(name).cloned()
    }

    pub fn get_all_units() -> NuclResult<Vec<SharedUnit>> {
        let guard = &*UNITS_REGISTRY.read()?;
        let val = guard.values().cloned().collect::<Vec<SharedUnit>>();
        Ok(val)
    }

    pub fn remove_unit(name: &String) -> NuclResult<()> {
        let mut g = UNITS_REGISTRY.write()?;
        let _ = g.remove_entry(name);
        Ok(())
    }

    pub fn add_unit(unit: SharedUnit) -> NuclResult<()> {
        let mut g = UNITS_REGISTRY.write()?;
        let name = unit.lock()?.get_name().clone();
        g.insert(name, unit);
        Ok(())
    }
}
///The registry of units that are running and that are not.
type RegType = HashMap<String, (SharedUnit, u32)>;
static ALREADY_RUNNING: LazyLock<RwLock<RegType>> = LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct RunningRegistry;

impl RunningRegistry {
    pub fn is_running(unit: SharedUnit) -> NuclResult<bool> {
        let guard = ALREADY_RUNNING.read()?;
        Ok(guard.contains_key(unit.lock()?.get_name()))
    }

    pub fn get_unit(name: &String) -> Option<SharedUnit> {
        let guard = ALREADY_RUNNING.read().ok()?;
        guard.get(name).map(|(unit, _)| Arc::clone(unit))
    }

    pub fn add_unit(unit: SharedUnit, pid: u32) -> NuclResult<()> {
        let mut guard = ALREADY_RUNNING.write()?;
        let g = unit.lock()?.get_name().clone();
        guard.insert(g, (unit, pid));
        Ok(())
    }

    pub fn remove_unit(unit: SharedUnit) -> NuclResult<()> {
        let mut guard = ALREADY_RUNNING.write()?;
        let _ = guard.remove(unit.lock()?.get_name());
        Ok(())
    }

    pub fn get_pid_of(unit: SharedUnit) -> Option<u32> {
        let w = ALREADY_RUNNING.read().ok()?;
        w.get(unit.lock().ok()?.get_name()).map(|(_, pid)| *pid)
    }
}
