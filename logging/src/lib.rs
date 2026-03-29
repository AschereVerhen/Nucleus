use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PidRecord {
    pub id: u32,
    pub unit_name: String,
    pub alive: bool,
    pub is_monitor: bool,
}

pub struct PidManager {
    file_path: PathBuf,
    /// DashMap allows concurrent, lock-free insertions and removals
    records: DashMap<u32, PidRecord>,
}

impl PidManager {
    #[instrument(skip(file_path), level = "trace")]
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        let file_path = file_path.as_ref().to_path_buf();
        let records = DashMap::new();

        if file_path.exists() {
            trace!("Loading existing PID records from disk");
            match File::open(&file_path) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    match serde_json::from_reader::<_, Vec<PidRecord>>(reader) {
                        Ok(disk_records) => {
                            for record in disk_records {
                                records.insert(record.id, record);
                            }
                            info!("Successfully loaded {} PID records", records.len());
                        }
                        Err(e) => warn!("Failed to parse pids.json, starting fresh: {}", e),
                    }
                }
                Err(e) => error!("Failed to open PID file: {}", e),
            }
        } else {
            debug!(
                "No existing PID file found at {:?}. Starting fresh.",
                file_path
            );
        }

        Self { file_path, records }
    }

    #[instrument(skip(self), fields(pid = record.id, unit = %record.unit_name))]
    pub fn register(&self, record: PidRecord) -> std::io::Result<()> {
        trace!("Registering new PID");
        self.records.insert(record.id, record);
        self.flush_to_disk()
    }

    #[instrument(skip(self), level = "debug")]
    pub fn mark_dead(&self, pid: u32) -> std::io::Result<()> {
        if let Some(mut record) = self.records.get_mut(&pid) {
            trace!(pid, "Marking PID as dead");
            record.alive = false;
        } else {
            warn!(pid, "Attempted to mark unknown PID as dead");
        }
        self.flush_to_disk()
    }

    #[instrument(skip(self), level = "debug")]
    pub fn remove(&self, pid: u32) -> std::io::Result<()> {
        if self.records.remove(&pid).is_some() {
            trace!(pid, "Removed PID from tracking");
            self.flush_to_disk()
        } else {
            warn!(pid, "Attempted to remove unknown PID");
            Ok(())
        }
    }

    /// Atomically flushes the DashMap to disk to prevent corruption
    #[instrument(skip(self), level = "trace")]
    fn flush_to_disk(&self) -> std::io::Result<()> {
        trace!("Flushing PID states to disk");

        // Convert map to vec for serialization
        let records_vec: Vec<PidRecord> =
            self.records.iter().map(|kv| kv.value().clone()).collect();
        let json_data = serde_json::to_string_pretty(&records_vec)?;

        let dir = self
            .file_path
            .parent()
            .expect("PID file must have a parent directory");
        fs::create_dir_all(dir)?;

        // Write to a temporary file first
        let mut temp_file = NamedTempFile::new_in(dir)?;
        temp_file.write_all(json_data.as_bytes())?;
        temp_file.flush()?;

        // Atomically rename over the old file
        temp_file.persist(&self.file_path)?;
        trace!("Flush complete");

        Ok(())
    }
}

