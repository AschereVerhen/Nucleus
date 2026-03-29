use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;

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

pub static NUCLD_HELPER_BINARIES: LazyLock<HashMap<String, PathBuf>> = LazyLock::new(|| {
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

pub struct DirPaths {
    pub user_dir: PathBuf,
    pub system_dir: PathBuf,
}

pub static DIRUNIT: LazyLock<DirPaths> = LazyLock::new(|| {
    let user_dir = if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(data_home).join("nuclinit/units")
    } else {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".local/share/nuclinit/units")
    };

    let system_dir = PathBuf::from("/etc/nuclinit/units");

    DirPaths {
        user_dir,
        system_dir,
    }
});
