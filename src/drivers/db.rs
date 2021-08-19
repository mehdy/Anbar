use std::collections::HashSet;

use crate::entities::bucket::Bucket;
use crate::entities::object::Object;
use crate::entities::user::User;

use serde_json::json;

#[derive(Clone)]
pub struct Db {
    db: sled::Db,
    access_key_to_user_id: sled::Tree,
    user_id_to_user: sled::Tree,
    user_id_to_bucket: sled::Tree,
    bucket_name_to_bucket: sled::Tree,
    bucket_name_to_objects: sled::Tree,
}

impl Db {
    pub fn new(path: &str) -> Self {
        let db = sled::open(path).unwrap();
        Self {
            access_key_to_user_id: db.open_tree("access_key_to_user_id").unwrap(),
            user_id_to_user: db.open_tree("user_id_to_user").unwrap(),
            user_id_to_bucket: db.open_tree("user_id_to_bucket").unwrap(),
            bucket_name_to_bucket: db.open_tree("bucket_name_to_bucket").unwrap(),
            bucket_name_to_objects: db.open_tree("bucket_name_to_objects").unwrap(),
            db: db,
        }
    }

    pub fn get_user_by_access_key(&self, access_key: &str) -> Option<User> {
        let user_id = self.access_key_to_user_id.get(access_key).unwrap()?;
        let user_buf = self.user_id_to_user.get(user_id).unwrap()?;
        let user: User = serde_json::from_slice(&user_buf).unwrap();
        Some(user)
    }

    pub fn create_user(&self, user: &User) {
        if self.user_id_to_user.get(&user.id).unwrap().is_some() {
            panic!("user already exists!");
        }

        self.user_id_to_user
            .insert(&user.id, serde_json::to_vec(&user).unwrap())
            .unwrap();
        self.user_id_to_bucket
            .insert(&user.id, serde_json::to_vec(&json!([])).unwrap())
            .unwrap();
        self.access_key_to_user_id
            .insert(&user.access_key, &user.id[..])
            .unwrap();
    }

    pub fn create_bucket(&self, bucket: &Bucket) {
        if self
            .bucket_name_to_bucket
            .get(&bucket.name)
            .unwrap()
            .is_some()
        {
            panic!("bucket already exists!")
        }

        self.bucket_name_to_bucket
            .insert(&bucket.name, serde_json::to_vec(bucket).unwrap())
            .unwrap();
        let user_buckets_buf = self
            .user_id_to_bucket
            .get(&bucket.owner_id)
            .unwrap()
            .unwrap();
        let mut user_buckets: Vec<Bucket> = serde_json::from_slice(&user_buckets_buf).unwrap();
        user_buckets.push(bucket.to_owned());
        self.user_id_to_bucket
            .insert(&bucket.owner_id, serde_json::to_vec(&user_buckets).unwrap())
            .unwrap();
    }

    pub fn get_buckets_by_user_id(&self, user_id: &str) -> HashSet<Bucket> {
        let user_buckets_buf = self.user_id_to_bucket.get(user_id).unwrap().unwrap();
        serde_json::from_slice(&user_buckets_buf).unwrap()
    }

    pub fn get_objects_by_bucket_name(&self, bucket_name: &str) -> HashSet<Object> {
        let objects_buf = self
            .bucket_name_to_objects
            .get(bucket_name)
            .unwrap()
            .unwrap_or(sled::IVec::from("[]"));
        serde_json::from_slice(&objects_buf).unwrap()
    }

    pub fn create_object(&self, object: &Object) {
        let mut objects = self.get_objects_by_bucket_name(&object.bucket);
        objects.insert(object.to_owned());

        self.bucket_name_to_objects
            .insert(&object.bucket, serde_json::to_vec(&objects).unwrap())
            .unwrap();
    }

    pub fn get_object(&self, bucket: &str, object: &str) -> Option<Object> {
        let objects = self.get_objects_by_bucket_name(bucket);

        objects.into_iter().find(|o| o.key == object)
    }

    pub fn delete_bucket(&self, bucket: &str) {
        self.bucket_name_to_bucket.remove(bucket).unwrap();
        self.bucket_name_to_objects.remove(bucket).unwrap();
    }

    pub fn delete_object(&self, bucket: &str, object: &str) {
        let mut objects = self.get_objects_by_bucket_name(bucket);
        objects.retain(|o| o.key == object);
        self.bucket_name_to_objects
            .insert(bucket, serde_json::to_vec(&objects).unwrap())
            .unwrap();
    }
}
