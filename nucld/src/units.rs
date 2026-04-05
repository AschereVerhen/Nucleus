use crate::prelude::*;
impl Exec for Unit {
    fn exec(&self) -> Result<u32, NuclErrors> {
        let shared = self.clone().shared();
        let dependencies = self.get_dependencies();
        if let Some(vec) = dependencies {
            for unit_name in vec {
                if RunningRegistry::is_running(shared.clone())? {
                    continue;
                }
                let deps_list = UnitRegistry::get_all_units()?;
                if let Some(indx) = deps_list
                    .iter()
                    .position(|n| n.lock().unwrap().get_name() == unit_name)
                {
                    deps_list[indx].clone().lock()?.exec()?;
                }
            }
        }
        let res = crate::exec::exec(shared)?;
        Ok(res)
    }
}
