use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::units::Unit;
use nuclerrors::NuclErrors;
use nuclerrors::NuclResult;
use walkdir::WalkDir;

// fn get_units() -> Vec<DirEntry> {
//     let dirs = &*crate::paths::DIRUNIT;
//     let dir = dirs.get_user_dir();
//     let mut units_user: Vec<DirEntry> = WalkDir::new(dir)
//         .into_iter()
//         .filter_map(|e| e.ok())
//         .collect();
//
//     units_user.extend(
//         WalkDir::new(dirs.get_system_dir())
//             .into_iter()
//             .filter_map(|e| e.ok())
//             .collect::<Vec<DirEntry>>(),
//     );
//     units_user
// }
fn get_units() -> NuclResult<Vec<PathBuf>> {
    //First check /etc/nuclinit/units/
    //Then we check /home/{} users.
    //check how many users we have
    //and then do USER.join(".local/nuclinit/units") and check for every units there....
    let entries = Arc::new(Mutex::new(Vec::new()));
    let system_dir = crate::paths::UnitDirs::get_system_dir();
    let mut threads = Vec::new();
    let entries_1 = entries.clone();
    threads.push(std::thread::spawn(move || -> NuclResult<()> {
        for entry in WalkDir::new(system_dir) {
            let e = entry?;
            if !e.path().is_dir() {
                entries_1.lock()?.push(e.path().to_path_buf())
            }
        }
        Ok(())
    }));
    let mut users = Vec::new();

    WalkDir::new("/home")
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.depth() == 1)
        .for_each(|f| users.push(f));

    for user in users {
        let path = user.path().join(".local/share/nuclinit/units");
        if !path.exists() {
            continue;
        }
        let entry = entries.clone();
        threads.push(std::thread::spawn(move || -> NuclResult<()> {
            for e in WalkDir::new(path) {
                let e = e?;
                if e.path().is_dir() {
                    continue;
                }
                entry.lock()?.push(e.path().to_path_buf())
            }
            Ok(())
        }));
    }

    for t in threads {
        t.join().unwrap()?
    }

    let e = Arc::try_unwrap(entries).unwrap().into_inner()?; //Safe because
    //there is only 1 owner here.
    Ok(e)
}

pub fn read_and_eval_units() -> NuclResult<Vec<Unit>> {
    let units = get_units()?;
    let mut vec = Vec::new();
    for unit in units.iter() {
        if unit.is_dir() {
            continue;
        }
        //Note the unwraps are safe here.
        let contents = std::fs::read_to_string(unit)?;
        let unitstruct: Unit = toml::from_str(&contents).unwrap();
        let filename_raw = unit.file_name().unwrap().as_bytes();
        let byte_period = &b"."[0];
        let file_name = if let Some(indx) = filename_raw.iter().position(|byte| byte == byte_period)
        {
            &filename_raw[0..indx] //manual truncation.
        } else {
            filename_raw
        };
        if unitstruct.get_name().as_bytes() != file_name {
            return Err(NuclErrors::NameMismatch {
                filename: unit.file_name().unwrap().to_string_lossy().to_string(),
            });
        }
        vec.push(unitstruct);
    }
    Ok(vec)
}
