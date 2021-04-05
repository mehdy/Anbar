use chrono::{DateTime, Local};

#[derive(Clone, Debug)]
pub struct Object {
    pub name: String,
    pub bucket: String,
    pub owner_id: String,
    pub size: i64,
    pub last_modified: DateTime<Local>,
}
