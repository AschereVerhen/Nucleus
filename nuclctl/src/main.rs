use clap::Parser;
use nucllib::commands::Commands;
use nucllib::errors::NuclErrors;
use nucllib::ipc::{IpcResponse, ResponseData};
use nucllib::units::Unit;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const SOCKET_PATH: &str = "/tmp/nucld.sock";

#[derive(Parser, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: Commands,
}

fn main() -> Result<(), NuclErrors> {
    // let _log_guard = nucllib::logging::init_logger("nuclctl");
    let s = Cmd::parse();
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    let input = serde_json::to_string(&s.cmd)?;
    println!("{}", &input);
    stream.write_all(input.as_bytes())?;
    //Now listen to the stream:
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let response: IpcResponse = serde_json::from_str(&response)?;

    let res = response.to_res()?;

    Ok(())
}

fn handle_response(val: ResponseData) -> Result<(), NuclErrors> {
    todo!()
}
