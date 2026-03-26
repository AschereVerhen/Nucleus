#![allow(dead_code)] //Since this project is not even complete yet, so yeah.

use crate::errors::{NuclErrors, extract_panic_message};
use crate::units::Unit;
use crate::{get_path_of, mark_name_as_running, thread, unmark_name_as_running};
use std::os::unix::process::CommandExt;
use std::process;

pub fn exec(unit: Unit) -> Result<u32, NuclErrors> {
    println!("exec at was called.");
    let monitor = unit.get_restart();
    let pid = if monitor {
        spawn_monitor(unit)?
    } else {
        exec_process(unit)?
    };
    Ok(pid)
}
fn exec_program(arguments: &[String]) -> Result<process::Child, NuclErrors> {
    let res = process::Command::new(&arguments[0])
        .args(&arguments[1..])
        .spawn()?;
    Ok(res)
}

fn exec_process(unit: Unit) -> Result<u32, NuclErrors> {
    let arguments = unit.get_cmd().to_vec();
    let name = unit.get_name().clone();

    let res = thread!(move || -> Result<u32, NuclErrors> {
        let mut child = exec_program(&arguments)?;
        let id = child.id();
        {
            mark_name_as_running(name.clone(), child.id())? //Takes ownership of child and stores it
            //on a hashmap.
        }
        thread!(move || -> Result<(), NuclErrors> {
            let _ = child.wait();
            unmark_name_as_running(&name)?;
            Ok(())
        })?;
        Ok(id)
    })?;
    match res.join() {
        Ok(val) => Ok(val?),
        Err(e) => Err(NuclErrors::ThreadPanic(extract_panic_message(e))),
    }
}

fn spawn_monitor(unit: Unit) -> Result<u32, NuclErrors> {
    let path_to_nuclstart = get_path_of(&"nuclstart".to_string())?;
    let serialized = serde_json::to_string(&unit)?;
    println!("{}", &serialized);
    let mut child = unsafe {
        process::Command::new(path_to_nuclstart)
            .arg("spawn-from-json")
            .arg("--command")
            .arg(serialized)
            .pre_exec(|| {
                nix::unistd::setsid()?;
                Ok(())
            })
            .spawn()?
    };
    let id = child.id();
    mark_name_as_running(unit.get_name().clone(), id)?;

    thread!(move || -> Result<(), NuclErrors> {
        let _ = child.wait();
        println!("Child died");
        unmark_name_as_running(unit.get_name())?;
        Ok(())
    })?;

    Ok(id)
}
