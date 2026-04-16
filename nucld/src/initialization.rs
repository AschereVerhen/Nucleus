use nuclconsts::units::{UnitBuilder, UserId};

//This file contains a few function to orcastrate the booting up sequence
//
use crate::prelude::*;

fn make_tmpfs() -> NuclResult<()> {
    //We will create dev, sys, proc, tmp and run
    let devtmpfs = UnitBuilder::new()
        .name("mount-dev".to_string())
        .user_defined(false)
        .dependencies(vec![])
        .restart(false)
        .runas(UserId::root())
        .cmd(vec![
            "mount".to_string(),
            "-t".to_string(),
            "devtmpfs".to_string(),
            "devtmpfs".to_string(),
            "/dev".to_string(),
        ])
        .autostart(true)
        .build()
        .shared();
    UnitRegistry::add_unit(devtmpfs.clone())?;
    devtmpfs.exec()?;

    let tmpfs = UnitBuilder::new()
        .name("mount-tmp".to_string())
        .user_defined(false)
        .dependencies(vec![])
        .restart(false)
        .runas(UserId::root())
        .cmd(vec![
            "mount".to_string(),
            "-t".to_string(),
            "tmpfs".to_string(),
            "tmpfs".to_string(),
            "/tmp".to_string(),
        ])
        .autostart(true)
        .build()
        .shared();
    UnitRegistry::add_unit(tmpfs.clone())?;
    tmpfs.exec()?;

    let runfs = UnitBuilder::new()
        .name("mount-run".to_string())
        .user_defined(false)
        .dependencies(vec![])
        .restart(false)
        .runas(UserId::root())
        .cmd(vec![
            "mount".to_string(),
            "-t".to_string(),
            "tmpfs".to_string(),
            "tmpfs".to_string(),
            "/run".to_string(),
        ])
        .autostart(true)
        .build()
        .shared();
    UnitRegistry::add_unit(runfs.clone())?;
    runfs.exec()?;

    let sysfs = UnitBuilder::new()
        .name("mount-tmp".to_string())
        .user_defined(false)
        .dependencies(vec![])
        .restart(false)
        .runas(UserId::root())
        .cmd(vec![
            "mount".to_string(),
            "-t".to_string(),
            "sysfs".to_string(),
            "sys".to_string(),
            "/sys".to_string(),
        ])
        .autostart(true)
        .build()
        .shared();
    UnitRegistry::add_unit(tmpfs.clone())?;
    sysfs.exec()?;

    Ok(())
}

fn execute_dbus() -> NuclResult<()> {
    std::fs::create_dir_all("/run/dbus/")?;
    let _ = Command::new("dbus-daemon").arg("--system").spawn()?;
    let dbus = UnitBuilder::new()
        .name("dbus".to_string())
        .user_defined(false)
        .dependencies(vec![])
        .restart(false)
        .runas(UserId::root())
        .cmd(vec!["dbus-daemon".to_string(), "--system".to_string()])
        .autostart(true)
        .build()
        .shared();
    UnitRegistry::add_unit(dbus.clone())?;
    dbus.exec()?;
    Ok(())
}

fn exec_agetty() -> NuclResult<()> {
    //Start 1 first..
    exec_agetty_on_ttys(1)?;

    let _ = std::thread::spawn(|| {
        for num in 2..=6 {
            let _ = exec_agetty_on_ttys(num);
        }
    });

    Ok(())
}

//num => [1, 6]
fn exec_agetty_on_ttys(num: u8) -> NuclResult<()> {
    let tty_path = format!("/dev/tty{}", num);
    let dbus = UnitBuilder::new()
        .name(format!("getty@tty{num}"))
        .user_defined(false)
        .dependencies(vec![])
        .restart(false)
        .runas(UserId::root())
        .cmd(vec!["/sbin/agetty".to_string(), tty_path])
        .autostart(true)
        .build()
        .shared();
    UnitRegistry::add_unit(dbus.clone())?;
    dbus.exec()?;
    Ok(())
}

pub fn prelude() -> NuclResult<()> {
    make_tmpfs()?;
    execute_dbus()?;
    exec_agetty()?;
    Ok(())
}
