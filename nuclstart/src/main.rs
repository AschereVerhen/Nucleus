use clap::{Parser, Subcommand};
use nuclconsts::units::Unit;
use nuclerrors::NuclErrors;
use nucllib::thread;
use std::process;

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

fn main() -> Result<(), NuclErrors> {
    let _log_guard = nucllib::logging::init_logger("nuclstart");
    let args = Arg::parse().cmd;
    match args {
        Subcommands::SpawnFromJson { json } => {
            let unit: Unit = serde_json::from_str(&json)?;
            spawn_monitor(unit)?; //let the main crate handle monitor or not.
        }
    };
    Ok(())
}

pub fn exec_program(arguments: &[String]) -> Result<process::Child, NuclErrors> {
    let res = process::Command::new(&arguments[0])
        .args(&arguments[1..])
        .spawn()?;
    Ok(res)
}
fn exec_monitor(unit: &Unit) -> Result<(), NuclErrors> {
    println!("exec_monitor invoked");
    let mut sleep_dur = 1;
    let argument = unit.get_cmd();
    loop {
        let res = exec_program(argument);
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

pub fn spawn_monitor(unit: Unit) -> Result<u32, NuclErrors> {
    let handle = thread!(move || -> Result<(), NuclErrors> {
        exec_monitor(&unit)?;
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
