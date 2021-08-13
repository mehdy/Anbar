use std::clone::Clone;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use chrono::Local;

use crate::drivers::db::Db;
use crate::entities::bucket::Bucket;
use crate::entities::object::Object;
use crate::entities::user::User;

#[derive(Clone)]
pub struct Storage {
    base_path: String,
    db: Db,
}

impl Storage {
    pub fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            db: Db::new(&format!("{}/.anbar.db", base_path)),
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

        self.db.create_user(&user);
    }

    pub fn find_user(&self, id: &str) -> Option<User> {
        self.db.get_user_by_access_key(id)
    }

    pub fn create_bucket(&mut self, owner_id: &str, name: &str) {
        let path = Path::new(&self.base_path).join(name);

        if !path.exists() {
            fs::create_dir(path).unwrap();
        }

        let bucket = Bucket {
            name: name.to_string(),
            owner_id: owner_id.to_string(),
            object_count: 0,
            size: 0,
            creation_date: Local::now(),
        };

        self.db.create_bucket(&bucket);
    }

    pub fn list_buckets(&self, owner_id: &str) -> Vec<Bucket> {
        self.db.get_buckets_by_user_id(owner_id)
    }

    pub fn list_objects(&self, bucket: &str) -> Vec<Object> {
        self.db.get_objects_by_bucket_name(bucket)
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

        let obj = Object {
            key: object.to_string(),
            bucket: bucket.to_string(),
            owner_id: user.id.to_string(),
            size: body.len() as i64,
            last_modified: Local::now(),
        };

        self.db.create_object(&obj);
    }

    pub fn get_object(&self, bucket: &str, object: &str, buf: &mut Vec<u8>) -> Object {
        let path = Path::new(&self.base_path).join(bucket).join(object);

        let mut file = File::open(path).unwrap();

        file.read_to_end(buf).unwrap();

        self.db.get_object(bucket, object).unwrap()
    }

    pub fn delete_bucket(&mut self, bucket: &str) {
        let path = Path::new(&self.base_path).join(bucket);
        fs::remove_dir_all(path).unwrap();

        self.db.delete_bucket(bucket);
    }

    pub fn delete_object(&mut self, bucket: &str, object: &str) {
        let path = Path::new(&self.base_path).join(bucket).join(object);
        fs::remove_file(path).unwrap();

        self.db.delete_object(bucket, object);
    }
}
