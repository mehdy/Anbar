use std::clone::Clone;
use std::convert::Infallible;
use std::sync::Arc;

use chrono::Utc;
use futures::TryStreamExt;
use hmac::Mac;
use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response, StatusCode};
use md5::{Digest, Md5};
use tokio::sync::Mutex;

use crate::adapters::bucket::ListAllMyBucketsResult;
use crate::adapters::object::ListBucketResult;
use crate::adapters::user::OwnerResult;
use crate::drivers::s3::{Auth, Operation};
use crate::entities::object::Object;
use crate::entities::user::User;
use crate::interactors::storage::Storage;

#[derive(Clone)]
pub struct App {
    pub storage: Arc<Mutex<Storage>>,
}

const AUTH_HEADER: &str = "Authorization";

impl App {
    fn get_auth_header(&self, req: &Request<Body>) -> String {
        req.headers()
            .get(AUTH_HEADER)
            .unwrap_or(&HeaderValue::from_static(""))
            .to_str()
            .unwrap_or("")
            .to_string()
    }

    fn check_signature(&self, user: &User, auth: &Auth, req: &Request<Body>) -> bool {
        let string_to_sign = auth.string_to_sign(&req);
        let mut key = auth.key_builder(&user.secret_access_key);
        key.update(string_to_sign.as_bytes());

        format!("{:x}", key.finalize().into_bytes()) == auth.signature
    }

    async fn list_buckets(&self, user: &User) -> Result<ListAllMyBucketsResult, String> {
        let storage = self.storage.lock().await;

        Ok(ListAllMyBucketsResult {
            buckets: storage
                .list_buckets(&user.id)
                .iter()
                .map(|b| b.into())
                .collect(),
            owner: OwnerResult {
                display_name: user.display_name.to_string(),
                id: user.id.to_string(),
            },
        })
    }

    async fn create_bucket(&self, user: &User, bucket: &str) -> Result<(), String> {
        let mut storage = self.storage.lock().await;
        Ok(storage.create_bucket(&user.id, bucket))
    }

    async fn list_objects(&self, bucket: &str) -> Result<ListBucketResult, String> {
        let storage = self.storage.lock().await;

        Ok(ListBucketResult {
            is_truncated: false,
            contents: storage
                .list_objects(bucket)
                .iter()
                .map(|o| o.into())
                .collect(),
            name: bucket.to_string(),
        })
    }

    async fn delete_bucket(&self, bucket: &str) -> Result<(), String> {
        let mut storage = self.storage.lock().await;
        Ok(storage.delete_bucket(&bucket))
    }

    async fn put_object(
        &self,
        user: &User,
        bucket: &str,
        key: &str,
        body: &[u8],
    ) -> Result<(), String> {
        let mut storage = self.storage.lock().await;

        storage.put_object(&user, &bucket, &key, body);

        Ok(())
    }

    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
        buf: &mut Vec<u8>,
    ) -> Result<Object, String> {
        let storage = self.storage.lock().await;

        Ok(storage.get_object(&bucket, &key, buf))
    }

    async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), String> {
        let mut storage = self.storage.lock().await;

        storage.delete_object(&bucket, &key);
        Ok(())
    }

    async fn find_user(&self, access_key: &str) -> Result<User, String> {
        let storage = self.storage.lock().await;

        Ok(storage.find_user(access_key).unwrap().to_owned())
    }

    pub async fn handle(self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let auth_str = self.get_auth_header(&req);
        let auth = Auth::parse(&auth_str);

        let user = self.find_user(&auth.access_key).await.unwrap();

        if auth_str != "" && !self.check_signature(&user, &auth, &req) {
            return Ok(Response::builder().status(401).body(Body::empty()).unwrap());
        }

        let result = match self.detect_operation(&req) {
            Operation::ListBuckets => Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(self.list_buckets(&user).await.unwrap().to_xml()))
                .unwrap(),
            Operation::CreateBucket(bucket) => {
                self.create_bucket(&user, &bucket).await.unwrap();
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
            }
            Operation::DeleteBucket(bucket) => {
                self.delete_bucket(&bucket).await.unwrap();

                Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap()
            }
            Operation::ListObjects(bucket) => Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(
                    self.list_objects(&bucket).await.unwrap().to_xml(),
                ))
                .unwrap(),
            Operation::PutObject(bucket, key) => {
                let entire_body = req
                    .into_body()
                    .try_fold(Vec::new(), |mut data, chunk| async move {
                        data.extend_from_slice(&chunk);
                        Ok(data)
                    })
                    .await
                    .unwrap();

                let mut hasher = Md5::new();
                hasher.update(&entire_body);
                let content_md5 = format!("{:x}", hasher.finalize());

                self.put_object(&user, &bucket, &key, &entire_body)
                    .await
                    .unwrap();
                Response::builder()
                    .status(StatusCode::OK)
                    .header("ETag", content_md5)
                    .body(Body::empty())
                    .unwrap()
            }
            Operation::GetObject(bucket, key) => {
                let mut buf = vec![];
                let object = self.get_object(&bucket, &key, &mut buf).await.unwrap();

                Response::builder()
                    .status(StatusCode::OK)
                    .header(
                        "Last-Modified",
                        object
                            .last_modified
                            .with_timezone(&Utc)
                            .format("%a, %d %b %Y %H:%M:%S GMT")
                            .to_string(),
                    )
                    .body(Body::from(buf))
                    .unwrap()
            }
            Operation::DeleteObject(bucket, key) => {
                self.delete_object(&bucket, &key).await.unwrap();

                Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap()
            }
        };

        Ok(result)
    }

    fn detect_operation(&self, req: &Request<Body>) -> Operation {
        let mut iter = req
            .uri()
            .path()
            .strip_prefix('/')
            .unwrap()
            .splitn(2, '/')
            .filter(|&c| c != "");
        let bucket = iter.next();
        let key = iter.next();

        match (req.method(), bucket, key) {
            (&Method::GET, Some(bucket), Some(key)) => {
                Operation::GetObject(bucket.to_string(), key.to_string())
            }
            (&Method::PUT, Some(bucket), Some(key)) => {
                Operation::PutObject(bucket.to_string(), key.to_string())
            }
            (&Method::DELETE, Some(bucket), Some(key)) => {
                Operation::DeleteObject(bucket.to_string(), key.to_string())
            }
            (&Method::PUT, Some(bucket), None) => Operation::CreateBucket(bucket.to_string()),
            (&Method::GET, Some(bucket), None) => Operation::ListObjects(bucket.to_string()),
            (&Method::DELETE, Some(bucket), None) => Operation::DeleteBucket(bucket.to_string()),
            (_, _, _) => Operation::ListBuckets,
        }
    }
}
