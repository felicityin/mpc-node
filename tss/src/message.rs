use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SyncMsg {
    pub type_:  SyncType,
    pub device: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum SyncType {
    #[default]
    Join = 0,
    Leave = 1,
    GenUuid = 2,
    Start = 3,
}
