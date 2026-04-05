use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

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

    if !user_dir.exists() {
        let _ = std::fs::create_dir_all(&user_dir);
    }
    if !system_dir.exists() {
        let _ = std::fs::create_dir_all(&system_dir);
    }

    DirPaths {
        user_dir,
        system_dir,
    }
});

pub static SOCKET_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from("/tmp/nuclinit/nucld.sock"));
