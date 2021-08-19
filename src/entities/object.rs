use std::hash::{Hash, Hasher};

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

impl PartialEq for Object {
    fn eq(&self, obj: &Object) -> bool {
        self.bucket == obj.bucket && self.key == obj.key
    }
}

impl Eq for Object {}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bucket.hash(state);
        self.key.hash(state);
    }
}
