#[derive(Clone, Debug)]
pub struct Object {
    pub name: String,
    pub bucket: String,
    pub owner_id: String,
    pub size: i64,
}
