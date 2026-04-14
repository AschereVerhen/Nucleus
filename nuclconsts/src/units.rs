use crate::units_parsing::read_and_eval_units;
use nix::unistd::Uid;
use nuclerrors::NuclErrors;
use nuclerrors::NuclResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::{LazyLock, RwLock};
use tabled::Tabled;
use walkdir::WalkDir;

#[derive(Default, Serialize, Deserialize, Debug, Tabled, Clone, Copy, Hash, PartialEq, Eq)]
pub struct UserId {
    uid: u32,
    gid: u32,
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}):({})", self.uid, self.gid)
    }
}

impl UserId {
    pub fn get_uid(&self) -> u32 {
        self.uid
    }
    pub fn get_gid(&self) -> u32 {
        self.gid
    }
    pub fn new(uid: u32, gid: u32) -> Self {
        Self { uid, gid }
    }
    pub fn is_root(&self) -> bool {
        self.uid == 0
    }
}

#[derive(Default)]
pub struct UnitBuilder {
    name: Option<String>,
    cmd: Option<Vec<String>>,
    restart: bool,
    dependencies: Vec<String>,
    autostart: bool,
    runas: Option<UserId>,
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

    pub fn runas(mut self, runas: UserId) -> Self {
        self.runas = Some(runas);
        self
    }

    pub fn build(self) -> Unit {
        Unit {
            name: self.name.ok_or("Name must be present").unwrap(),
            cmd: self.cmd.unwrap_or(vec![]),
            restart: self.restart,
            autostart: self.autostart,
            dependencies: Some(self.dependencies),
            runas: self.runas.unwrap_or(UserId::default()),
        }
    }
}

fn format_cmd(v: &[String]) -> String {
    v.join(" ")
}

fn format_optional_vec(opt: &Option<Vec<String>>) -> String {
    match opt {
        Some(v) if !v.is_empty() => v.join(", "),
        _ => "None".to_string(),
    }
}

fn default_value_runas() -> UserId {
    UserId::default()
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
    #[serde(default = "default_value_runas")]
    runas: UserId, //Pid
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
    pub fn get_runas(&self) -> UserId {
        self.runas
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
    pub fn write_unit(unit_struct: SharedUnit) -> NuclResult<()> {
        let dirs = crate::paths::UnitDirs::get_system_dir();
        let user = unit_struct.lock()?.get_runas();
        let user = nix::unistd::User::from_uid(Uid::from(user.get_uid()))?.unwrap(); //Note: if the unit_struct
        //exists, that means the conversion from "String" -> u32 has already been done AND user is
        //sure to exist. So unwrapping() here is safe.
        let e = user.dir.join(".local/share/nuclinit/units");

        let dir = if user.uid != Uid::from(0) {
            if !e.exists() {
                std::fs::create_dir_all(&e)?;
            }
            e.as_path()
        } else {
            &dirs
        };

        let new_unit_file = { dir.join(format!("{}.toml", unit_struct.lock()?.get_name())) };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(new_unit_file)?;
        let serialized = { toml::to_string_pretty(&*unit_struct.lock()?) }?; //drop the lock
        //asap.
        writeln!(&mut file, "{serialized}")?;
        Ok(())
    }

    pub fn remove_unit(unit_name: String) -> NuclResult<()> {
        let dirs = &*crate::paths::UnitDirs::get_system_dir();
        let unit = UnitRegistry::get_unit(&unit_name);

        if unit.is_none() {
            return Err(NuclErrors::UnitIsInvalid { name: unit_name });
        }

        let unit = unit.unwrap();

        let user = unit.lock()?.get_runas();
        let user = nix::unistd::User::from_uid(Uid::from(user.get_uid()))?.unwrap(); //Safe because unit
        //exists. and the extended reasoning is written above
        let e = user.dir.join(".local/share/nuclinit/units");

        let dir = if user.uid != Uid::from(0) {
            if !e.exists() {
                std::fs::create_dir_all(&e)?;
            }
            e.as_path()
        } else {
            dirs
        };

        let target_name = std::ffi::OsString::from(format!("{}.toml", unit_name));

        for e in WalkDir::new(dir) {
            let e = e?;
            if e.file_name() == target_name {
                std::fs::remove_file(e.path())?
            }
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
    pub fn get_unit(name: &str) -> Option<SharedUnit> {
        let guard = UNITS_REGISTRY.read().ok()?;
        guard.get(name).cloned()
    }

    pub fn get_all_units() -> NuclResult<Vec<SharedUnit>> {
        let guard = &*UNITS_REGISTRY.read()?;
        let val = guard.values().cloned().collect::<Vec<SharedUnit>>();
        Ok(val)
    }

    pub fn remove_unit(name: &str) -> NuclResult<()> {
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

    pub fn get_unit(name: &str) -> Option<SharedUnit> {
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
