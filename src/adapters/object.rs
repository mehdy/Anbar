use chrono::{DateTime, Local};

use crate::adapters::user::OwnerResult;
use crate::entities::object::Object;

#[derive(Debug)]
pub struct ObjectResult {
    etag: String,
    key: String,
    owner: OwnerResult,
    size: i64,
    last_modified: DateTime<Local>,
}

impl From<&Object> for ObjectResult {
    fn from(object: &Object) -> Self {
        Self {
            etag: "".to_string(),
            key: object.name.to_string(),
            owner: OwnerResult {
                id: object.owner_id.to_string(),
                display_name: "".to_string(),
            },
            size: object.size,
            last_modified: object.last_modified,
        }
    }
}

impl ObjectResult {
    pub fn to_xml(&self) -> String {
        format!(
            "<Contents><ETag>{}</ETag><Key>{}</Key>{}<Size>{}</Size><LastModified>{:?}</LastModified></Contents>",
            self.etag,
            self.key,
            self.owner.to_xml(),
            self.size,
            self.last_modified,
        )
    }
}

#[derive(Debug)]
pub struct ListBucketResult {
    pub is_truncated: bool,
    pub contents: Vec<ObjectResult>,
    pub name: String,
}

impl ListBucketResult {
    pub fn to_xml(&self) -> String {
        format!(
            "<ListBucketResult><IsTruncated>{}</IsTruncated>{}<Name>{}</Name></ListBucketResult>",
            self.is_truncated,
            self.contents
                .iter()
                .map(|c| c.to_xml())
                .collect::<Vec<String>>()
                .join(""),
            self.name
        )
    }
}
