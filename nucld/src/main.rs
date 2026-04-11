use fs2::FileExt;
use nix::unistd::{ForkResult, Pid};
use nuclconsts::paths::SOCKET_PATH;
use nucld::parse_input::execute_command;
use nucld::prelude::*;
use nuclerrors::NuclResult;
use nucllib::ipc::IpcResponse;
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, instrument, trace, warn};

fn daemonize() -> NuclResult<u32> {
    let fork_res = unsafe { nix::unistd::fork()? };
    match fork_res {
        ForkResult::Child => {
            nix::unistd::setsid()?;
            Ok(0u32)
        }
        ForkResult::Parent { child } => {
            let pid_child = child.as_raw();
            Ok(pid_child as u32)
        }
    }
}

fn main() -> NuclResult<()> {
    if !is_root() {
        panic!("Run the init manager as root.");
    }
    let _lock_file = PathBuf::from("/tmp/nuclinit/nucld.lock");
    if !_lock_file.parent().unwrap().exists() {
        std::fs::create_dir_all(_lock_file.parent().unwrap())?;
    }
    let _file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(_lock_file)?;
    match _file.try_lock_exclusive() {
        Ok(()) => (),
        Err(_) => {
            //this means that another instance of nucld is already running. hence we will return
            //from the main function.
            return Ok(());
        }
    }

    // let res = daemonize()?;
    // if res != 0 {
    //     info!(pid=%res, "Daemonized nucld with "); //pid = pid ofc
    //     return Ok(()); //kill the parent thread;
    // }

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

    //Start autostart
    thread!(|| -> NuclResult<()> {
        nucld::autostart::autostart_units()?;
        Ok(())
    })?;

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
fn handle_client(mut stream: UnixStream) -> NuclResult<()> {
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
