use nix::sys::signal::Signal::SIGKILL;
use nix::unistd::{ForkResult, Pid, setsid};
use nuclcommands::Commands;
use nucld::errors::NuclErrors;
use nucld::get_unit_from_name;
use nucld::thread;
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use tabled::settings::Style;

const SOCKET_PATH: &str = "/tmp/nucld.sock";

fn main() -> Result<(), NuclErrors> {
    //daemonize_self().unwrap();

    let _ = std::fs::remove_file(SOCKET_PATH); //Delete previous file 

    let listener = UnixListener::bind(SOCKET_PATH)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream)?,
            Err(e) => eprintln!("An error occured: {e:?}"),
        };
    }
    Ok(())
}
#[allow(unused)]
fn daemonize_self() -> Result<(), NuclErrors> {
    let res = unsafe { nix::unistd::fork()? };
    match res {
        ForkResult::Parent { child } => {
            println!("Child pid: {}", child.as_raw());
            std::process::exit(0);
        }
        _ => {
            setsid()?;
        }
    }
    Ok(())
}

fn handle_client(mut stream: UnixStream) -> Result<(), NuclErrors> {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let input = String::from_utf8_lossy(&buffer[..size]);
            let cmd: Commands = serde_json::from_str(&input)?;
            println!("{:?}", cmd);
            execute_command(cmd)?;
        }
        Err(e) => eprintln!("An error occured: {e:?}"),
    }
    Ok(())
}

#[allow(unused_variables)]
fn execute_command(cmd: Commands) -> Result<(), NuclErrors> {
    match cmd {
        Commands::Enable { name } => {
            //add_to_rebootstart_list(name) //TODO: Complete this function
            todo!();
        }
        Commands::Disable { name } => {
            //remove_from_rebootstart_list(name); //TODO: complete this function aswell.
            todo!();
        }
        Commands::Start { name } => {
            let unit = nucld::get_unit_from_name(&name);
            if let Some(u) = unit {
                println!("This must execute.");
                thread!(move || -> Result<(), NuclErrors> { u.exec() })?;
                println!("Does this execute?");
            }
        }
        Commands::Stop { name } => {
            let res = nucld::get_pid_of(&name);
            println!(
                "Stop invoked with the name: {}, and res returned: {:?}",
                &name, res
            );
            if let Ok(pid) = res {
                println!("The pid is: {}", pid);
                let pid = Pid::from_raw(pid as i32);
                let restart = get_unit_from_name(&name).unwrap().get_restart();
                //if its restart: Then we do killpg, else we do kill.
                if restart {
                    println!("Killing process group: {:?}", pid);
                    nix::sys::signal::killpg(pid, SIGKILL)?;
                } else {
                    println!("Killing process: {:?}", pid);
                    nix::sys::signal::kill(pid, SIGKILL)?;
                }
            }
        }
        Commands::ListUnits => {
            let units = nucld::get_units();
            let mut table = tabled::Table::new(units.iter().map(|u| u.as_ref()));
            table.with(Style::modern_rounded());
            println!("{}", table)
        }
        Commands::AddUnit {
            name,
            cmd,
            restart,
            autostart,
            dependencies,
        } => {
            nucld::units::write_unit(name, cmd, restart, autostart, dependencies)?;
        }
        Commands::RemoveUnit { name } => {
            todo!()
        }
        Commands::Status { name } => {
            todo!()
        }
    }
    Ok(())
}
