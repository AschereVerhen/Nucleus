use crate::is_root;
use crate::units::Unit;
use nuclerrors::NuclErrors;
use nuclerrors::NuclResult;
use nuclerrors::extract_panic_message;
use walkdir::{DirEntry, WalkDir};

fn get_units() -> Vec<DirEntry> {
    let dirs = &*crate::paths::DIRUNIT;
    let dir = &dirs.user_dir;
    let mut units_user: Vec<DirEntry> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    let res = if is_root() {
        units_user.extend(
            WalkDir::new(&dirs.system_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .collect::<Vec<DirEntry>>(),
        );
        units_user
    } else {
        units_user
    };
    println!("Res is: {res:?}");
    res
}

pub fn read_and_eval_units() -> NuclResult<Vec<Unit>> {
    let handle = std::thread::spawn(|| -> NuclResult<Vec<Unit>> {
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
