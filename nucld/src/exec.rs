use crate::prelude::*;
use std::os::unix::process::CommandExt;

pub trait Exec {
    fn exec(&self) -> Result<(), NuclErrors>;
}

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
use tracing::{debug, error, info, instrument, trace};

#[instrument(skip(unit), fields(unit_name = %unit.get_name()), level = "debug")]
fn exec_process(unit: Unit) -> Result<u32, NuclErrors> {
    info!("Attempting to execute unit process");

    let arguments = unit.get_cmd().to_vec();
    let name = unit.get_name().clone();

    trace!(?arguments, "Extracted unit execution arguments");

    let res = thread!(move || -> Result<u32, NuclErrors> {
        debug!("Spawning child process for unit");
        let mut child = exec_program(&arguments)?;
        let id = child.id();
        info!(child_pid = id, "Successfully spawned process");

        {
            trace!("Marking unit as running in ALREADY_RUNNING map");
            mark_name_as_running(name.clone(), child.id())?;
        }

        // Waiter thread
        thread!(move || -> Result<(), NuclErrors> {
            trace!(child_pid = id, "Waiter thread actively monitoring process");
            match child.wait() {
                Ok(status) => info!(child_pid = id, exit_status = ?status, "Child process exited"),
                Err(e) => error!(child_pid = id, error = ?e, "Failed waiting on child process"),
            }

            trace!(unit_name = %name, "Unmarking unit from ALREADY_RUNNING");
            unmark_name_as_running(&name)?;
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
