use crate::prelude::*;
use nuclerrors::NuclResult;
use std::collections::{HashMap, HashSet};

fn resolve_dependencies(unit_name: &str) -> NuclResult<Vec<String>> {
    let registry = UnitRegistry::get_all_units()?;

    // Build graph
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for unit in registry.iter() {
        let (name, deps) = {
            let guard = unit.lock()?;
            let name = guard.get_name().clone();
            let deps = guard
                .get_dependencies()
                .map(|d| d.to_vec())
                .unwrap_or_default();
            std::mem::drop(guard);
            (name, deps)
        };

        graph.insert(name, deps);
    }

    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();
    let mut order = Vec::new();

    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> NuclResult<()> {
        if visited.contains(node) {
            return Ok(());
        }

        if visiting.contains(node) {
            return Err(NuclErrors::UnitIsInvalid {
                name: format!("Cycle detected at {}", node),
            });
        }

        visiting.insert(node.to_string());

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                dfs(dep, graph, visited, visiting, order)?;
            }
        }
        visiting.remove(node);
        visited.insert(node.to_string());
        order.push(node.to_string());

        Ok(())
    }

    dfs(unit_name, &graph, &mut visited, &mut visiting, &mut order)?;

    Ok(order)
}
impl Exec for SharedUnit {
    fn exec(&self) -> NuclResult<u32> {
        let unit_name = {
            let guard = self.lock()?;
            guard.get_name().clone()
        };

        let execution_order = resolve_dependencies(&unit_name)?;

        let mut last_pid = 0;

        for unit_name in execution_order {
            let unit = UnitRegistry::get_unit(&unit_name).ok_or(NuclErrors::UnitIsInvalid {
                name: unit_name.clone(),
            })?;

            let is_running = RunningRegistry::is_running(unit.clone())?;

            if is_running {
                continue;
            }

            let pid = crate::exec::exec(unit.clone())?;

            last_pid = pid;
        }

        Ok(last_pid)
    }
}
