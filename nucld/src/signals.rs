use crate::prelude::*;
use nix::{sys::signal, unistd::Pid};
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::System;

pub static GOT_TERMINATE: AtomicBool = AtomicBool::new(false);
pub static GOT_SIGCHLD: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigterm(_: i32) {
    GOT_TERMINATE.store(true, Ordering::SeqCst);
}

extern "C" fn reap_children(_: i32) {
    GOT_SIGCHLD.store(true, Ordering::SeqCst);
}

extern "C" fn ignore_sigint(_: i32) {}

extern "C" fn ignore_sigquit(_: i32) {}

pub fn handle_signals() -> NuclResult<()> {
    //Handler for sigterm:
    unsafe { signal::signal(signal::SIGTERM, signal::SigHandler::Handler(handle_sigterm))? }; //Need
    //to pass a functional pointer-equivalent.
    //Handle SIGCHILD:
    unsafe { signal::signal(signal::SIGCHLD, signal::SigHandler::Handler(reap_children))? };
    //Ignore SIGINT and SIGQUIT
    unsafe { signal::signal(signal::SIGINT, signal::SigHandler::Handler(ignore_sigint))? };
    unsafe { signal::signal(signal::SIGQUIT, signal::SigHandler::Handler(ignore_sigquit))? };
    Ok(())
}

pub fn terminate(howto: nix::sys::reboot::RebootMode) -> NuclResult<()> {
    //This function is to trigger a termination of every units...
    signal::kill(Pid::from_raw(-1), signal::SIGTERM)?;
    std::thread::sleep(std::time::Duration::from_secs(5));
    let processes = System::new().processes().len();
    if processes != 1 {
        signal::kill(Pid::from_raw(-1), signal::SIGKILL)?;
    };

    let _ = std::fs::remove_file("/tmp/nuclinit/nucld.lock");

    nix::sys::reboot::reboot(howto)?;

    Ok(())
}
