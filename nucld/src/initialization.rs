use std::os::unix::process::CommandExt;

//This file contains a few function to orcastrate the booting up sequence
//
use crate::prelude::*;

fn make_tmpfs() -> NuclResult<()> {
    //We will create dev, sys, proc, tmp and run
    let _ = Command::new("mount")
        .args(["-t", "devtmpfs", "devtmpfs", "/dev"])
        .status()?;
    let _ = Command::new("mount")
        .args(["-t", "proc", "proc", "/proc"])
        .status()?;
    let _ = Command::new("mount")
        .args(["-t", "tmpfs", "tmpfs", "/run"])
        .status()?;
    let _ = Command::new("mount")
        .args(["-t", "tmpfs", "tmpfs", "/tmp"])
        .status()?;
    let _ = Command::new("mount")
        .args(["-t", "sysfs", "sys", "/sys"])
        .status()?;

    Ok(())
}

fn execute_dbus() -> NuclResult<()> {
    std::fs::create_dir_all("/run/dbus/")?;
    let _ = Command::new("dbus-daemon").arg("--system").spawn()?;

    Ok(())
}

fn exec_agetty_on_ttys() -> NuclResult<()> {
    for num in 0..9 {
        let tty = format!("/dev/tty{}", num);
        let _ = unsafe {
            Command::new("/sbin/agetty")
                .args([tty, "115200".into(), "linux".into()])
                .pre_exec(|| {
                    nix::unistd::setsid()?;
                    Ok(())
                })
                .spawn()?
        };
    }
    Ok(())
}

pub fn prelude() -> NuclResult<()> {
    make_tmpfs()?;
    execute_dbus()?;
    exec_agetty_on_ttys()?;
    Ok(())
}
