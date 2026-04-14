use crate::prelude::*;
use tracing::{info, instrument, trace};

#[instrument(level = "info")]
pub fn autostart_units() -> NuclResult<()> {
    info!("Starting autostart units.");
    let units: Vec<SharedUnit> = UnitRegistry::get_all_units()?
        .iter()
        .filter(|f| {
            let g = f.lock().unwrap();
            trace!(unit = %*g.get_name(), "Evaluating");
            g.get_autostart()
        })
        .cloned()
        .collect();
    for unit in units {
        info!(unit = unit.lock()?.get_name(), "Execing");
        unit.exec()?;
    }
    Ok(())
}
