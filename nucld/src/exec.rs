use crate::prelude::*;
use nuclerrors::NuclResult;
use std::os::unix::process::CommandExt;

pub trait Exec {
    fn exec(&self) -> NuclResult<u32>;
}

pub fn exec(unit: SharedUnit) -> Result<u32, NuclErrors> {
    let (monitor, uid) = {
        let g = unit.lock()?;
        (g.get_autostart(), g.get_runas())
    };

    if uid != 0 {
        let pid = spawn_as_non_root(unit.clone(), uid)?;
        return Ok(pid);
    }

    let pid = if monitor {
        spawn_monitor(unit.clone())?
    } else {
        exec_process(unit.clone())?
    };
    RunningRegistry::add_unit(unit, pid)?;
    Ok(pid)
}
fn exec_program(arguments: &[String], uid: u32) -> Result<process::Child, NuclErrors> {
    let res = process::Command::new(&arguments[0])
        .args(&arguments[1..])
        .uid(uid)
        .gid(uid)
        .spawn()?;
    Ok(res)
}
use tracing::{debug, error, info, instrument, trace};

#[instrument(skip(unit), fields(unit_name = %unit.lock()?.get_name()), level = "debug")]
fn exec_process(unit: SharedUnit) -> Result<u32, NuclErrors> {
    info!("Attempting to execute unit process");

    let (arguments, name, uid) = {
        let guard = unit.lock()?;
        (
            guard.get_cmd().to_vec(),
            guard.get_name().clone(),
            guard.get_runas(),
        )
    };

    trace!(?arguments, "Extracted unit execution arguments");

    let res = thread!(move || -> Result<u32, NuclErrors> {
        debug!("Spawning child process for unit");
        let mut child = exec_program(&arguments, uid)?;
        let id = child.id();
        info!(child_pid = id, "Successfully spawned process");

        {
            trace!("Marking unit as running in ALREADY_RUNNING map");
            RunningRegistry::add_unit(Arc::clone(&unit), child.id())?;
        }

        // Waiter thread
        thread!(move || -> Result<(), NuclErrors> {
            trace!(child_pid = id, "Waiter thread actively monitoring process");
            match child.wait() {
                Ok(status) => info!(child_pid = id, exit_status = ?status, "Child process exited"),
                Err(e) => error!(child_pid = id, error = ?e, "Failed waiting on child process"),
            }

            trace!(unit_name = %name, "Unmarking unit from ALREADY_RUNNING");
            RunningRegistry::remove_unit(unit)?;
            Ok(())
        })?;

        Ok(id)
    })?;

    match res.join() {
        Ok(val) => {
            debug!("Unit execution thread returned successfully");
            Ok(val?)
        }
        Err(e) => {
            let panic_msg = extract_panic_message(e);
            error!(panic_message = %panic_msg, "Unit execution thread panicked");
            Err(NuclErrors::ThreadPanic(panic_msg))
        }
    }
}

///Will invoke nuclstart and make it start the program..
fn spawn_monitor(unit: SharedUnit) -> Result<u32, NuclErrors> {
    let path_to_nuclstart = get_path_of(&"nuclstart".to_string())?;
    let unit_cloned = Arc::clone(&unit);
    let serialized = serde_json::to_string(unit_cloned.as_ref())?;
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
    RunningRegistry::add_unit(unit.clone(), id)?;
    thread!(move || -> Result<(), NuclErrors> {
        let _ = child.wait();
        RunningRegistry::remove_unit(unit)?;
        Ok(())
    })?;

    Ok(id)
}

fn spawn_as_non_root(unit: SharedUnit, uid: u32) -> Result<u32, NuclErrors> {
    let path_to_nuclstart = get_path_of(&"nuclstart".to_string())?;
    let unit_cloned = Arc::clone(&unit);
    let serialized = serde_json::to_string(unit_cloned.as_ref())?;
    let mut child = unsafe {
        process::Command::new(path_to_nuclstart)
            .arg("spawn-from-json")
            .arg("--command")
            .arg(serialized)
            .uid(uid) //THE ONLY CHANGE!
            .gid(uid)
            .pre_exec(|| {
                nix::unistd::setsid()?;
                Ok(())
            })
            .spawn()?
    };
    let id = child.id();
    RunningRegistry::add_unit(unit.clone(), id)?;
    thread!(move || -> Result<(), NuclErrors> {
        let _ = child.wait();
        RunningRegistry::remove_unit(unit)?;
        Ok(())
    })?;

    Ok(id)
}
