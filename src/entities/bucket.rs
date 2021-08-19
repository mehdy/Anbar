use std::hash::{Hash, Hasher};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub owner_id: String,
    pub object_count: i64,
    pub size: i64,
    pub creation_date: DateTime<Local>,
}

impl PartialEq for Bucket {
    fn eq(&self, obj: &Bucket) -> bool {
        self.name == obj.name
    }
}

impl Eq for Bucket {}

impl Hash for Bucket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
