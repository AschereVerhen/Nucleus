use serde::{Deserialize, Serialize};

use nuclerrors::{NuclErrors, NuclResult};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum ResponseData {
    Empty,
    JsonResponse(String),
    Number(u32),
    Pid(u32),

    // system-specific
    UnitStarted { pid: u32 },
    UnitStopped,
    UnitStatus { running: bool },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", content = "data")]
pub enum IpcResponse {
    Ok(ResponseData),
    Err(NuclErrors),
}

impl IpcResponse {
    pub fn to_res(self) -> NuclResult<ResponseData> {
        match self {
            IpcResponse::Ok(v) => Ok(v),
            IpcResponse::Err(e) => Err(e),
        }
    }
    pub fn from_res(val: NuclResult<ResponseData>) -> Self {
        match val {
            Ok(r) => IpcResponse::Ok(r),
            Err(e) => IpcResponse::Err(e),
        }
    }
}
