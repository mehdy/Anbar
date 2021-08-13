use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub access_key: String,
    pub secret_access_key: String,
}
