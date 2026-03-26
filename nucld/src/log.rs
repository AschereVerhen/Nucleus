#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use tabled::Tabled;

use crate::units::Unit;
#[derive(Deserialize, Serialize, Debug, Tabled, Default)]
pub struct PidRecord {
    #[tabled(display = "display_unit")]
    unit: Unit,
    id: u32,
    alive: bool,
    monitor: bool,
}

fn display_unit(unit: &Unit) -> String {
    format!("{:#?}", unit)
}

impl PidRecord {
    pub fn get_unit(&self) -> &Unit {
        &self.unit
    }
    pub fn get_id(&self) -> u32 {
        self.id
    }
    pub fn is_alive(&self) -> bool {
        self.alive
    }
    pub fn is_monitor(&self) -> bool {
        self.monitor
    }
    pub fn set_alive(&mut self) {
        self.alive = true
    }
    pub fn set_dead(&mut self) {
        self.alive = false
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PidFile {
    pids: Vec<PidRecord>,
}

impl PidFile {
    pub fn get_mut_pids(&mut self) -> &mut Vec<PidRecord> {
        &mut self.pids
    }
}

pub static FILEPATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let val = dirs::data_dir();
    if val.is_none() {
        let data_dir =
            PathBuf::from(std::env::var("HOME").expect("Expected HOME to be set. Aborting."))
                .join(".local/share/nuclstart");
        std::fs::create_dir_all(data_dir).expect("Failed to create the paths");
    };
    let path = dirs::data_dir().unwrap().join("nuclstart/pids.json"); //This time this will be safe.
    if !path.exists() {
        std::fs::File::create(&path).expect("Failed to create path");
    }

    let contents = std::fs::read_to_string(&path).expect("failed to read from the file.");
    if contents.is_empty() || *crate::FIRST_RUN.read().expect("Failed to get lock") {
        let default_val_for_json = r#"{"pids": []}"#;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(false)
            .truncate(true)
            .open(&path)
            .expect("Failed to create the File Struct.");
        writeln!(&mut file, "{default_val_for_json}").expect("Failed to clear the file.");
    }
    *crate::FIRST_RUN.write().expect("Failed to get lock") = false;
    path
});

pub fn write_to_file(pidstruct: PidFile) -> Result<(), std::io::Error> {
    let content = serde_json::to_string_pretty(&pidstruct)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true)
        .open(&*FILEPATH)?;
    writeln!(&mut file, "{content}")?;
    Ok(())
}

pub fn log_process_pid(pid: u32, unit: Unit) -> std::io::Result<()> {
    let file = &*FILEPATH;
    let contents = std::fs::read_to_string(file)?;
    let mut pids: PidFile = serde_json::from_str(&contents)?;
    let records = pids.get_mut_pids();
    let mut new_pid_record = PidRecord::default();
    new_pid_record.id = pid;
    new_pid_record.monitor = false;
    new_pid_record.alive = true;
    new_pid_record.unit = unit;
    records.push(new_pid_record);
    write_to_file(pids)?;
    Ok(())
}

pub fn log_monitor_pid(pid: u32, unit: Unit) -> std::io::Result<()> {
    let file = &*FILEPATH;
    let contents = std::fs::read_to_string(file)?;
    let mut pids: PidFile = serde_json::from_str(&contents)?;
    let records = pids.get_mut_pids();
    let mut new_pid_record = PidRecord::default();
    new_pid_record.id = pid;
    new_pid_record.monitor = true;
    new_pid_record.alive = true;
    new_pid_record.unit = unit;
    records.push(new_pid_record);
    write_to_file(pids)?;
    Ok(())
}

pub fn dead_process_pid(pid: u32) -> std::io::Result<()> {
    let file = &*FILEPATH;
    let contents = std::fs::read_to_string(file)?;
    let mut pids: PidFile = serde_json::from_str(&contents)?;
    let records = pids.get_mut_pids();
    for (index, record) in records.iter().enumerate() {
        if record.id == pid {
            records.remove(index);
            break;
        }
    }
    write_to_file(pids)?;
    Ok(())
}
