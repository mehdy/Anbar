use std::clone::Clone;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use chrono::Local;

use crate::entities::bucket::Bucket;
use crate::entities::object::Object;
use crate::entities::user::User;

#[derive(Clone)]
pub struct Storage {
    base_path: String,
    buckets: Vec<Bucket>,
    objects: Vec<Object>,
    users: Vec<User>,
}

impl Storage {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            buckets: vec![],
            objects: vec![],
            users: vec![],
        }
    }

    pub fn new_user(
        &mut self,
        id: &str,
        display_name: &str,
        access_key: &str,
        secret_access_key: &str,
    ) {
        let user = User {
            id: id.to_string(),
            display_name: display_name.to_string(),
            access_key: access_key.to_string(),
            secret_access_key: secret_access_key.to_string(),
        };

        self.users.push(user);
    }

    pub fn find_user(&self, id: &str) -> Option<&User> {
        self.users.iter().find(|&u| u.access_key == id)
    }

    pub fn create_bucket(&mut self, owner_id: &str, name: &str) {
        let path = Path::new(&self.base_path).join(name);

        if !path.exists() {
            fs::create_dir(path).unwrap();
        }

        self.buckets.push(Bucket {
            name: name.to_string(),
            owner_id: owner_id.to_string(),
            object_count: 0,
            size: 0,
            creation_date: Local::now(),
        });
    }

    pub fn list_buckets(&self, owner_id: &str) -> Vec<&Bucket> {
        let user = self.users.iter().find(|&u| u.id == owner_id).unwrap();
        self.buckets
            .iter()
            .filter(|&b| b.owner_id == user.id)
            .collect()
    }

    pub fn list_objects(&self, bucket: &str) -> Vec<&Object> {
        self.objects
            .iter()
            .filter(|&o| o.bucket == bucket)
            .collect()
    }

    pub fn put_object(&mut self, user: &User, bucket: &str, object: &str, body: &[u8]) {
        let path = Path::new(&self.base_path).join(bucket).join(object);

        let mut file = File::with_options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .unwrap();
        file.write_all(body).unwrap();
        file.sync_all().unwrap();

        self.objects.push(Object {
            key: object.to_string(),
            bucket: bucket.to_string(),
            owner_id: user.id.to_string(),
            size: body.len() as i64,
            last_modified: Local::now(),
        })
    }

    pub fn get_object(&self, bucket: &str, object: &str, buf: &mut Vec<u8>) -> &Object {
        let path = Path::new(&self.base_path).join(bucket).join(object);

        let mut file = File::open(path).unwrap();

        file.read_to_end(buf).unwrap();

        self.objects
            .iter()
            .find(|&o| o.bucket == bucket && o.key == object)
            .unwrap()
    }
}
