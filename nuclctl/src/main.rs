use clap::Parser;
use nuclconsts::paths::SOCKET_PATH;
use nuclconsts::units::Unit;
use nuclerrors::NuclResult;
use nucllib::commands::Commands;
use nucllib::ipc::{IpcResponse, ResponseData};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use tabled::settings::Modify;
use tabled::settings::object::Rows;

#[derive(Parser, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: Commands,
}

fn main() -> NuclResult<()> {
    // let _log_guard = nucllib::logging::init_logger("nuclctl");
    let s = Cmd::parse();
    let mut stream = UnixStream::connect(&*SOCKET_PATH)?;
    let input = serde_json::to_string(&s.cmd)?;
    stream.write_all(input.as_bytes())?;
    //Now listen to the stream:
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let response: IpcResponse = serde_json::from_str(&response)?;

    let res = response.to_res()?;
    match res {
        ResponseData::JsonResponse(s) => {
            let result: Vec<Unit> = serde_json::from_str(&s)?;
            let table = create_table(result);
            println!("{}", table);
        }
        ResponseData::Empty => (),
        ResponseData::UnitStarted { pid } => {
            println!("Unit successfully started with pid: {}", pid)
        }
        ResponseData::UnitStopped => {
            println!("Unit successfully stopped.")
        }
        ResponseData::UnitStatus { running } => println!(
            "Unit is {}",
            if running { "running" } else { "not running" }
        ),
        _ => todo!(),
    };

    Ok(())
}

fn create_table<T>(data: T) -> tabled::Table
where
    T: std::iter::IntoIterator,
    T::Item: tabled::Tabled,
{
    let mut table = tabled::Table::builder(data).index().build();
    table.with(tabled::settings::Style::modern_rounded());
    table.with(Modify::new(Rows::first()).with(tabled::settings::Color::FG_BRIGHT_GREEN));
    table
}
