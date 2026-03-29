use crate::prelude::*;
impl Exec for Unit {
    fn exec(&self) -> Result<(), NuclErrors> {
        let dependencies = self.get_dependencies();
        if let Some(vec) = dependencies {
            for unit_name in vec {
                if query_if_name_is_running(unit_name)? {
                    continue;
                }
                let deps_list = read_and_eval_units()?;
                if let Some(indx) = deps_list.iter().position(|n| n.get_name() == unit_name) {
                    deps_list[indx].clone().exec()?;
                }
            }
        }
        crate::exec::exec(self.clone())?;
        Ok(())
    }
}
