use crate::{prelude::*, signals::terminate};
use nix::sys::signal::SIGKILL;
use nix::unistd::Pid;
use nuclconsts::units::{UnitBuilder, UserId};
use nucllib::ipc::ResponseData;
use tracing::{Span, debug, info, instrument, trace, warn};

#[instrument(level = "info", skip_all, fields(command = ?cmd))]
pub fn execute_command(cmd: Commands) -> NuclResult<ResponseData> {
    match cmd {
        Commands::Start { name } => {
            let span = Span::current();
            span.record("unit", name.as_str());

            info!(unit = %name, "Received Start command");
            let unit = UnitRegistry::get_unit(&name);
            if let Some(u) = unit.clone() {
                debug!(unit = %name, "Unit found, spawning execution thread");
                u.exec()?;
            } else {
                warn!(unit = %name, "Start failed: Unit not found");
                return Err(NuclErrors::UnitIsInvalid { name });
            };
            let unit = unit.unwrap();
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
            debug!("Generating unit table");

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
            runas,
        } => {
            info!(unit = %name, command = ?cmd, "Adding new unit to database");
            // Values passed to the helper function
            let user_id = {
                let id = nix::unistd::User::from_name(&runas)?;
                if id.is_none() {
                    return Err(NuclErrors::UserNotFound { name: runas });
                }
                let id = id.unwrap();
                UserId::new(id.uid.as_raw(), id.gid.as_raw())
            };
            let unit_struct = UnitBuilder::new()
                .name(name.clone())
                .cmd(cmd)
                .restart(restart)
                .dependencies(dependencies.unwrap_or(vec![]))
                .autostart(autostart)
                .runas(user_id)
                .build()
                .shared();
            UnitFS::write_unit(unit_struct.clone())?;
            UnitRegistry::add_unit(unit_struct)?;
            Ok(ResponseData::Empty)
        }
        Commands::RemoveUnit { name } => {
            info!(unit = %name, "Removing a unit from system");
            UnitFS::remove_unit(name.clone())?;
            UnitRegistry::remove_unit(&name)?;
            Ok(ResponseData::Empty)
        }
        Commands::Status { name } => {
            info!(unit = %name, "Sending the status of a unit.");
            let unit = UnitRegistry::get_unit(&name);
            if unit.is_none() {
                return Err(NuclErrors::UnitIsInvalid { name });
            }
            let unit = unit.unwrap();
            Ok(ResponseData::UnitStatus {
                running: RunningRegistry::is_running(unit)?,
            })
        }
        Commands::Enable { name } => {
            let unit = UnitRegistry::get_unit(&name);

            if unit.is_none() {
                return Err(NuclErrors::UnitIsInvalid { name });
            }
            let unit = unit.unwrap();

            unit.lock()?.set_autostart(true);

            UnitFS::write_unit(unit)?;

            Ok(ResponseData::Empty)
        }
        Commands::Disable { name } => {
            let unit = UnitRegistry::get_unit(&name);

            if unit.is_none() {
                return Err(NuclErrors::UnitIsInvalid { name });
            }
            let unit = unit.unwrap();

            unit.lock()?.set_autostart(false);

            UnitFS::write_unit(unit)?;

            Ok(ResponseData::Empty)
        }
        Commands::Poweroff => {
            terminate(nix::sys::reboot::RebootMode::RB_POWER_OFF)?;
            Ok(ResponseData::Empty)
        }
        Commands::Reboot => {
            terminate(nix::sys::reboot::RebootMode::RB_AUTOBOOT)?;
            Ok(ResponseData::Empty)
        }
    }
}
