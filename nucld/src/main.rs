use nix::sys::signal::SIGKILL;
use nix::unistd::Pid;
use nuclconsts::paths::SOCKET_PATH;
use nuclconsts::units::UnitBuilder;
use nucld::prelude::*;
use nucllib::ipc::{IpcResponse, ResponseData};
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use tracing::{Span, debug, error, info, instrument, trace, warn};

fn main() -> Result<(), NuclErrors> {
    // Initialize the logger we built in the previous step
    let _log_guard = nucllib::logging::init_logger("nucld");

    info!("Initializing nucld daemon");

    if SOCKET_PATH.exists() {
        trace!(
            "Cleaning up existing domain socket at {}",
            &SOCKET_PATH.display()
        );
        let _ = std::fs::remove_file(&*SOCKET_PATH);
    }

    if let Some(parent) = SOCKET_PATH.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?
    }
    if is_root() {
        debug!("Running as root: spawning zombie reaper thread");
        thread!(|| {
            loop {
                reap_children();
                std::thread::sleep(std::time::Duration::from_secs(120));
            }
        })?;
    } else {
        warn!("Daemon not running as root. Some process management features may fail.");
    }

    let listener = UnixListener::bind(&*SOCKET_PATH).map_err(|e| {
        error!(error = %e, "Failed to bind to Unix socket at {}", SOCKET_PATH.display());
        e
    })?;

    info!(path = %SOCKET_PATH.display(), "nucld listening for commands");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // We don't want a client error to crash the whole daemon loop
                if let Err(e) = handle_client(stream) {
                    error!(error = ?e, "Error handling client request");
                }
            }
            Err(e) => error!(error = %e, "Incoming connection failed"),
        };
    }
    Ok(())
}

#[instrument(level = "debug")]
fn reap_children() {
    use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
    trace!("Starting child process reaping cycle");
    loop {
        match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => break,
            Ok(status) => {
                info!(wait_status = ?status, "Successfully reaped child process");
            }
            Err(nix::errno::Errno::ECHILD) => {
                trace!("No children left to reap");
                break;
            }
            Err(err) => {
                error!(error = %err, "Error occurred during waitpid");
                break;
            }
        }
    }
}

#[instrument(skip(stream), level = "info")]
fn handle_client(mut stream: UnixStream) -> Result<(), NuclErrors> {
    let mut buffer = [0; 1024];
    trace!("Reading data from client stream");

    let size = stream.read(&mut buffer).map_err(|e| {
        error!(error = %e, "Failed to read from client socket");
        e
    })?;

    if size == 0 {
        debug!("Received empty request from client");
        return Ok(());
    }

    let input = String::from_utf8_lossy(&buffer[..size]);
    debug!(raw_input = %input, "Parsing client command");

    let cmd: Commands = serde_json::from_str(&input).map_err(|e| {
        error!(error = %e, input = %input, "Failed to deserialize command");
        e
    })?;

    // The 'cmd' fields will be automatically included in all logs within execute_command
    let response = execute_command(cmd);
    let response = match response {
        Ok(r) => IpcResponse::Ok(r),
        Err(e) => IpcResponse::Err(e),
    };
    let serialized = serde_json::to_string(&response)?;
    stream.write_all(serialized.as_bytes())?;
    Ok(())
}

// #[instrument(level = "info", skip_all, fields(command = ?cmd))]
fn execute_command(cmd: Commands) -> Result<ResponseData, NuclErrors> {
    match cmd {
        Commands::Start { name } => {
            let span = Span::current();
            span.record("unit", name.as_str());

            info!(unit = %name, "Received Start command");
            let unit = UnitRegistry::get_unit(&name);
            let thread;
            if let Some(u) = unit.clone() {
                debug!(unit = %name, "Unit found, spawning execution thread");
                thread = thread!(move || -> Result<(), NuclErrors> {
                    let guard = u.lock().inspect_err(|_| {
                        error!("Failed to lock unit mutex");
                    })?;
                    info!(unit = %guard.get_name(), "Executing unit process");
                    guard.exec()?;
                    Ok(())
                })?;
            } else {
                warn!(unit = %name, "Start failed: Unit not found");
                return Err(NuclErrors::UnitNotFound { name });
            };
            let unit = unit.unwrap();
            let _ = thread.join();
            let pid = RunningRegistry::get_pid_of(unit);
            if pid.is_none() {
                panic!("An undefined behaviour has occured.");
            }
            let pid = pid.unwrap();
            Ok(ResponseData::UnitStarted { pid })
        }

        Commands::Stop { name } => {
            info!(unit = %name, "Received Stop command");
            let unit = RunningRegistry::get_unit(&name);
            if unit.is_none() {
                return Err(NuclErrors::UnitNotRunning { name });
            }
            let unit = unit.unwrap();
            let res = RunningRegistry::get_pid_of(unit.clone());

            match res {
                Some(pid_val) => {
                    let pid = Pid::from_raw(pid_val as i32);
                    let restart = unit.lock()?.get_restart();

                    if restart {
                        info!(pid = ?pid, "Sending SIGKILL to process group (restart enabled)");
                        nix::sys::signal::killpg(pid, SIGKILL)?;
                    } else {
                        info!(pid = ?pid, "Sending SIGKILL to specific process");
                        nix::sys::signal::kill(pid, SIGKILL)?;
                    }
                }
                None => warn!(unit = %name, "Stop failed: PID not found for unit"),
            }
            Ok(ResponseData::UnitStopped)
        }

        Commands::ListUnits => {
            debug!("Generating unit status table");
            let units = UnitRegistry::get_all_units()?;
            let mut cloned_unit = Vec::new();

            //Need this to convert Vec<Arc<Mutex<Unit>>> -> Vec<Unit>
            for unit in units {
                {
                    let guard = unit.lock()?;
                    cloned_unit.push(guard.clone());
                }
            }

            let serialized = serde_json::to_string(&cloned_unit)?;
            trace!(table = %serialized, "Units json generated");
            Ok(ResponseData::JsonResponse(serialized))
        }

        Commands::AddUnit {
            name,
            cmd,
            restart,
            autostart,
            dependencies,
            user,
        } => {
            info!(unit = %name, command = ?cmd, "Adding new unit to database");
            // Values passed to the helper function
            let unit_struct = UnitBuilder::new()
                .name(name.clone())
                .cmd(cmd)
                .restart(restart)
                .dependencies(dependencies.unwrap_or(vec![]))
                .autostart(autostart)
                .build();
            UnitFS::write_unit(unit_struct.shared(), user)?;
            Ok(ResponseData::Empty)
        }
        Commands::RemoveUnit { name } => {
            info!(unit = %name, "Removing a unit from system");
            UnitFS::remove_unit(name.clone())?;
            UnitRegistry::remove_unit(&name)?;
            Ok(ResponseData::Empty)
        }
        _ => {
            warn!("Command variant not yet implemented or unknown");
            todo!();
        }
    }
}
