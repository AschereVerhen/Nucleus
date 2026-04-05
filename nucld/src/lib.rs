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
