#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::{path::PathBuf, sync::LazyLock};
use tabled::Tabled;
use walkdir::{DirEntry, WalkDir};

use crate::errors::{NuclErrors, extract_panic_message};
use crate::query_if_name_is_running as query_name;

///Returns directory of the unit files.
static DIRUNIT: LazyLock<PathBuf> = LazyLock::new(|| {
    let priv_dir = PathBuf::from("/etc/nuclstart/units");
    let user_dir = PathBuf::from(&*crate::log::FILEPATH) //Need to wrap this
        //into a new PathBuf to take ownership of it.
        .parent()
        .expect("There should always be a parent. This is defined behaviour.")
        .join("units");
    if *crate::IS_ROOT {
        std::fs::create_dir_all(&priv_dir).expect("Failed to create root directory");
        priv_dir
    } else {
        std::fs::create_dir_all(&user_dir).expect("Failed to create user directory");
        user_dir
    }
});

fn get_units() -> Vec<DirEntry> {
    let dir = &*DIRUNIT;
    let units: Vec<DirEntry> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();
    units
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
            let filename_raw = unit.file_name().as_bytes();
            let byte_period = &b"."[0];
            let file_name =
                if let Some(indx) = filename_raw.iter().position(|byte| byte == byte_period) {
                    &filename_raw[0..indx] //manual truncation.
                } else {
                    filename_raw
                };
            if unitstruct.get_name().as_bytes() != file_name {
                panic!("The name of the unit must be same as the filename of the unit.")
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
    restart: bool,
    autostart: bool,
    #[tabled(display = "format_optional_vec")]
    dependencies: Option<Vec<String>>, //File names.
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
    pub fn build(self) -> Self {
        self
    }
    pub fn autostart(&self) -> bool {
        self.autostart
    }

    pub fn exec(&self) -> Result<(), NuclErrors> {
        let dependencies = self.get_dependencies();
        if let Some(vec) = dependencies {
            for unit_name in vec {
                if query_name(unit_name)? {
                    continue;
                }
                let deps_list = read_and_eval_units()?;
                if let Some(indx) = deps_list.iter().position(|n| n.name == unit_name.clone()) {
                    deps_list[indx].clone().exec()?;
                }
            }
        }
        crate::exec::exec(self.clone())?;
        Ok(())
    }
}
pub fn write_unit(
    name: String,
    cmd: Vec<String>,
    restart: bool,
    autostart: bool,
    dependencies: Option<Vec<String>>,
) -> std::io::Result<()> {
    let unit_struct = UnitBuilder::new()
        .name(name.clone())
        .cmd(cmd)
        .restart(restart)
        .dependencies(dependencies.unwrap_or(vec![]))
        .autostart(autostart)
        .build();

    let dir = &*DIRUNIT;
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
