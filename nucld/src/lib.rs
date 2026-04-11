pub mod autostart;
pub mod exec;
pub mod parse_input;
pub mod prelude;
pub mod units;

use crate::prelude::*;

pub fn get_path_of(name: &String) -> NuclResult<PathBuf> {
    if let Some(path) = NUCLD_HELPER_BINARIES.get(name) {
        Ok(path.clone())
    } else {
        Err(NuclErrors::BinaryNotFound { name: name.clone() })
    }
}
