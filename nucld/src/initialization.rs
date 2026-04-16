use std::{
    fs::OpenOptions,
    os::{fd::AsRawFd, unix::process::CommandExt},
};

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
        let tty_path = format!("/dev/tty{}", num);
        let tty = OpenOptions::new().read(true).write(true).open(&tty_path)?;
        unsafe {
            Command::new("/sbin/agetty")
                .args([tty_path, "115200".into(), "linux".into()])
                .pre_exec(move || {
                    nix::unistd::setsid()?;
                    let fd = tty.as_raw_fd();
                    nix::libc::ioctl(fd, nix::libc::TIOCSCTTY, 0);

                    nix::libc::dup2(fd, 0);
                    nix::libc::dup2(fd, 1);
                    nix::libc::dup2(fd, 2);

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
