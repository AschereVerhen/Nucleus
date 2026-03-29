//This file is for the "enable" and "disable" commands handling.
//
use crate::prelude::*;

pub fn set_autostart_for_unit(name: &String, autostart: bool) -> Result<(), NuclErrors> {
    let unit = get_unit_from_name(name);
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
