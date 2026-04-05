//This file is for the "enable" and "disable" commands handling.
//
use crate::prelude::*;

pub fn set_autostart_for_unit(name: &String, autostart: bool) -> Result<(), NuclErrors> {
    let unit = UnitRegistry::get_unit(name);
    if unit.is_none() {
        Err(NuclErrors::UnitIsInvalid { name: name.clone() })?;
    }
    let unit = unit.unwrap();
    {
        let mut guard = unit.lock()?;
        guard.set_autostart(autostart);
    }
    Ok(())
}

pub fn autostart_units() -> Result<(), NuclErrors> {
    let units: Vec<SharedUnit> = UnitRegistry::get_all_units()?
        .iter()
        .cloned()
        .filter(|f| f.lock().unwrap().get_autostart())
        .collect();
    for unit in units {
        unit.lock()?.exec()?;
    }
    Ok(())
}
