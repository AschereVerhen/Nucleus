use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Eq, Hash, PartialEq, EnumIter)]
pub enum HelperBins {
    NuclD,
    NuclCtl,
    NuclStart,
}

impl std::fmt::Display for HelperBins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HelperBins::NuclD => write!(f, "nucld"),
            HelperBins::NuclCtl => write!(f, "nuclctl"),
            HelperBins::NuclStart => write!(f, "nuclstart"),
        }
    }
}

// static NUCLINIT_HELPER_BINARIES: LazyLock<HashMap<HelperBins, PathBuf>> = LazyLock::new(|| {
//     let mut map = HashMap::new();
//     for bin in HelperBins::iter() {
//         if let Ok(path) = which::which(bin.to_string()) {
//             map.insert(bin, path);
//         }
//     }
//
//     map
// });

pub struct HelperBinsRegistry;

impl HelperBinsRegistry {
    pub fn get_path_of(name: HelperBins) -> Option<PathBuf> {
        match name {
            HelperBins::NuclD => Some("/usr/local/bin/nucld".into()),
            HelperBins::NuclStart => Some("/usr/local/bin/nuclstart".into()),
            HelperBins::NuclCtl => Some("/usr/local/bin/nuclctl".into()),
        }
    }
}

pub struct UnitDirs;

impl UnitDirs {
    pub fn get_system_dir() -> PathBuf {
        DIRUNIT.clone().to_path_buf()
    }
}

static DIRUNIT: LazyLock<PathBuf> = LazyLock::new(|| {
    let system_dir = PathBuf::from("/etc/nuclinit/units");

    if !system_dir.exists() {
        let _ = std::fs::create_dir_all(&system_dir);
    }

    system_dir
});

static SOCKET_PATHS: LazyLock<HashMap<HelperBins, PathBuf>> = LazyLock::new(|| {
    let mut hashmap = HashMap::new();
    for bin in HelperBins::iter() {
        let path = PathBuf::from(format!("/tmp/nuclinit/{}.sock", bin));
        hashmap.insert(bin, path);
    }
    hashmap
});

pub struct SocketRegistry;

impl SocketRegistry {
    pub fn get_path_of(h: HelperBins) -> PathBuf {
        let g = &*SOCKET_PATHS;
        g.get(&h).cloned().unwrap() //Sure to be there.
    }
}
