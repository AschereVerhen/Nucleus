use crate::prelude::*;
use nuclerrors::NuclResult;
use std::os::unix::process::CommandExt;
use tracing::{debug, info, instrument};

pub trait Exec {
    fn exec(&self) -> NuclResult<u32>;
}
#[instrument(level = "info", skip(unit))]
pub fn exec(unit: SharedUnit) -> Result<u32, NuclErrors> {
    let (monitor, name) = {
        let g = unit.lock()?;
        (g.get_autostart(), g.get_name().clone())
    };

    //for loggging:
    tracing::Span::current().record("unit_name", name.as_str());
    tracing::Span::current().record("monitor", monitor);

    debug!("Evaluating execution strategy");

    let pid = exec_process(unit.clone())?;

    info!(pid = pid, "Registering running unit");
    RunningRegistry::add_unit(unit, pid)?;
    Ok(pid)
}
fn exec_process(unit: SharedUnit) -> Result<u32, NuclErrors> {
    let path_to_nuclstart = HelperBinsRegistry::get_path_of(HelperBins::NuclStart).unwrap();
    let unit_cloned = Arc::clone(&unit);
    let serialized = serde_json::to_string(unit_cloned.as_ref())?;
    let mut child = unsafe {
        process::Command::new(path_to_nuclstart)
            .arg("spawn-from-json")
            .arg("--command")
            .arg(serialized)
            .pre_exec(|| {
                nix::unistd::setsid()?;
                Ok(())
            })
            .spawn()?
    };
    let id = child.id();
    RunningRegistry::add_unit(unit.clone(), id)?;
    thread!(move || -> Result<(), NuclErrors> {
        let _ = child.wait();
        RunningRegistry::remove_unit(unit)?;
        Ok(())
    })?;

    Ok(id)
}
