use clap::{Parser, Subcommand};
use nuclconsts::units::{Unit, UserId};
use nucld::prelude::*;
use nuclerrors::NuclErrors;
use nucllib::thread;
use std::{os::unix::process::CommandExt, process};
use tracing::{debug, error, info, instrument, trace};

#[derive(Subcommand, Clone)]
enum Subcommands {
    SpawnFromJson {
        #[arg(long = "command", short)]
        json: String,
    },
}

#[derive(Parser)]
struct Arg {
    #[command(subcommand)]
    cmd: Subcommands,
}

fn main() -> NuclResult<()> {
    let args = Arg::parse().cmd;
    match args {
        Subcommands::SpawnFromJson { json } => {
            let _log_guard = nucllib::logging::init_logger("nuclstart");
            let unit: Unit = serde_json::from_str(&json)?;
            let runas = unit.get_runas();
            if unit.get_autostart() {
                spawn_monitor(unit, runas)?;
            } else {
                exec_process(&unit, runas)?;
            }
        }
    };
    Ok(())
}

pub fn exec_program(arguments: &[String], id: UserId) -> NuclResult<process::Child> {
    let envs = build_envs()?;
    let res = process::Command::new(&arguments[0])
        .args(&arguments[1..])
        .gid(id.get_gid())
        .uid(id.get_uid())
        .envs(envs)
        .spawn()?;
    Ok(res)
}
fn exec_monitor(unit: &Unit, id: UserId) -> NuclResult<()> {
    let mut sleep_dur = 1;
    let argument = unit.get_cmd();
    loop {
        let res = exec_program(argument, id);
        if res.is_err() {
            std::thread::sleep(std::time::Duration::from_secs(sleep_dur));
            sleep_dur *= 2;
            continue;
        }
        let mut child = res?;
        let _ = child.wait(); //This is blocking and when code goes ahead, the process died and
        //should be revived.
        sleep_dur = 1;
    }
}

pub fn spawn_monitor(unit: Unit, id: UserId) -> NuclResult<u32> {
    let handle = thread!(move || -> NuclResult<()> {
        exec_monitor(&unit, id)?;
        Ok(())
    })?;
    match handle.join() {
        Ok(_) => Ok(()),
        Err(e) => Err(NuclErrors::ThreadPanic(nuclerrors::extract_panic_message(
            e,
        ))),
    }?;
    Ok(std::process::id())
}

#[instrument(skip(unit), fields(unit_name = %unit.get_name()), level = "debug")]
fn exec_process(unit: &Unit, id: UserId) -> NuclResult<u32> {
    info!("Attempting to execute unit process");

    let (arguments, name) = { (unit.get_cmd().to_vec(), unit.get_name().clone()) };

    trace!(?arguments, "Extracted unit execution arguments");
    let shared_unit = unit.clone().shared();

    let res = thread!(move || -> NuclResult<u32> {
        debug!("Spawning child process for unit");
        let mut child = exec_program(&arguments, id)?;
        let id = child.id();
        info!(child_pid = id, "Successfully spawned process");

        trace!("Marking unit as running in ALREADY_RUNNING map");
        RunningRegistry::add_unit(shared_unit.clone(), child.id())?;

        // Waiter thread
        thread!(move || -> NuclResult<()> {
            trace!(child_pid = id, "Waiter thread actively monitoring process");
            match child.wait() {
                Ok(status) => info!(child_pid = id, exit_status = ?status, "Child process exited"),
                Err(e) => error!(child_pid = id, error = ?e, "Failed waiting on child process"),
            }

            trace!(unit_name = %name, "Unmarking unit from ALREADY_RUNNING");
            RunningRegistry::remove_unit(shared_unit.clone())?;
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

macro_rules! hashmap {
    ($($key:expr => $val:expr,)* $(,)?) => {
        {
            let mut hashmap_new = std::collections::HashMap::new();
            $(
                hashmap_new.insert($key.to_string(), $val.to_string());
            )*
            hashmap_new
        }
    };
}

fn build_envs() -> NuclResult<HashMap<String, String>> {
    let hashmap = hashmap!(
        "USER" => "aschere",
        "HOME" => "/home/aschere/",
        "PATH" => "/usr/local/bin:/usr/bin:/bin",
        "DBUS_SESSION_BUS_ADDRESS" => "unix:path=/run/user/1000/bus",
    );

    Ok(hashmap)
}
