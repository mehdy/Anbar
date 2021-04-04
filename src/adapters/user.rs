use crate::entities::user::User;

#[derive(Debug)]
pub struct OwnerResult {
    pub id: String,
    pub display_name: String,
}

impl From<&User> for OwnerResult {
    fn from(user: &User) -> OwnerResult {
        Self {
            id: user.id.to_string(),
            display_name: user.display_name.to_string(),
        }
    }
}

impl OwnerResult {
    pub fn to_xml(&self) -> String {
        format!(
            "<Owner><DisplayName>{}</DisplayName><ID>{}</ID></Owner>",
            self.display_name, self.id
        )
    }
}
