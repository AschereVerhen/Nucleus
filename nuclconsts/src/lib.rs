use nix::unistd;
use nuclerrors::NuclResult;
use std::sync::{LazyLock, RwLock};

pub mod paths;
pub mod units;
mod units_parsing;

static FIRST_RUN: RwLock<bool> = RwLock::new(true);
pub fn is_first_run() -> NuclResult<bool> {
    Ok(*FIRST_RUN.read()?)
}
pub fn set_first_run(val: bool) -> NuclResult<()> {
    let mut guard = FIRST_RUN.write()?;
    *guard = val;
    Ok(())
}

static IS_ROOT: LazyLock<bool> = LazyLock::new(|| unistd::Uid::effective().is_root());

pub fn is_root() -> bool {
    *IS_ROOT
}
