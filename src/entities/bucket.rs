use chrono::{DateTime, Local};

#[derive(Clone, Debug)]
pub struct Bucket {
    pub name: String,
    pub owner_id: String,
    pub object_count: i64,
    pub size: i64,
    pub creation_date: DateTime<Local>,
}
