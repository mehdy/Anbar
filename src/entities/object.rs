use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub key: String,
    pub bucket: String,
    pub owner_id: String,
    pub size: i64,
    pub last_modified: DateTime<Local>,
}
