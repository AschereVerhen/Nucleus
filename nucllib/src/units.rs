use crate::errors::{NuclErrors, extract_panic_message};
use nuclconsts::{self, IS_ROOT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use tabled::Tabled;
use walkdir::{DirEntry, WalkDir};

pub static UNITS: LazyLock<HashMap<String, SharedUnit>> = LazyLock::new(|| {
    let vec: Vec<Unit> = read_and_eval_units().expect("Failed to eval units");
    let mut hashmap = HashMap::new();
    for unit in vec.into_iter() {
        hashmap.insert(unit.get_name().clone(), Arc::new(Mutex::new(unit)));
    }
    hashmap
});
fn get_units() -> Vec<DirEntry> {
    let dirs = &*nuclconsts::paths::DIRUNIT;
    let dir = &dirs.user_dir;
    let mut units_user: Vec<DirEntry> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    if *IS_ROOT {
        units_user.extend(
            WalkDir::new(&dirs.system_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .collect::<Vec<DirEntry>>(),
        );
        units_user
    } else {
        units_user
    }
}

pub fn read_and_eval_units() -> Result<Vec<Unit>, NuclErrors> {
    let handle = std::thread::spawn(|| -> Result<Vec<Unit>, NuclErrors> {
        let units = get_units();
        let mut vec = Vec::new();
        for unit in units.iter() {
            if unit.path().is_dir() {
                continue;
            }

            let contents = std::fs::read_to_string(unit.path())?;
            let unitstruct: Unit = toml::from_str(&contents).unwrap();
            let filename_raw = unit.file_name().as_encoded_bytes();
            let byte_period = &b"."[0];
            let file_name =
                if let Some(indx) = filename_raw.iter().position(|byte| byte == byte_period) {
                    &filename_raw[0..indx] //manual truncation.
                } else {
                    filename_raw
                };
            if unitstruct.get_name().as_bytes() != file_name {
                return Err(NuclErrors::NameMismatch {
                    filename: unit.file_name().to_string_lossy().to_string(),
                });
            }
            vec.push(unitstruct);
        }
        Ok(vec)
    });
    match handle.join() {
        Ok(res) => res,
        Err(panic) => Err(NuclErrors::ThreadPanic(extract_panic_message(panic))),
    }
}

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

#[derive(Deserialize, Serialize, Debug, Clone, Tabled, Default)]
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

pub type SharedUnit = Arc<Mutex<Unit>>;

fn format_cmd(v: &[String]) -> String {
    v.join(" ")
}

fn format_optional_vec(opt: &Option<Vec<String>>) -> String {
    match opt {
        Some(v) if !v.is_empty() => v.join(", "),
        _ => "none".to_string(),
    }
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
}
pub fn write_unit(
    name: String,
    cmd: Vec<String>,
    restart: bool,
    autostart: bool,
    dependencies: Option<Vec<String>>,
    user: bool,
) -> Result<(), NuclErrors> {
    let unit_struct = UnitBuilder::new()
        .name(name.clone())
        .cmd(cmd)
        .restart(restart)
        .dependencies(dependencies.unwrap_or(vec![]))
        .autostart(autostart)
        .build();

    let dirs = &*nuclconsts::paths::DIRUNIT;

    let dir = if user {
        &dirs.user_dir
    } else {
        if !*IS_ROOT {
            return Err(NuclErrors::INITIsNotRoot);
        }

        &dirs.system_dir
    };

    let new_unit_file = dir.join(format!("{}.toml", name));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(new_unit_file)?;
    let serialized = toml::to_string_pretty(&unit_struct).unwrap();
    writeln!(&mut file, "{serialized}")?;
    Ok(())
}
