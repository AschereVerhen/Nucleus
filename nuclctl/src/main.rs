#![allow(dead_code, unused)]

use clap::Parser;
use nuclcommands::Commands;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const SOCKET_PATH: &str = "/tmp/nucld.sock";

#[derive(Parser, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: Commands,
}

fn main() -> std::io::Result<()> {
    let s = Cmd::parse();
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    let input = serde_json::to_string(&s.cmd)?;
    println!("{}", &input);
    stream.write_all(input.as_bytes())?;

    Ok(())
}
