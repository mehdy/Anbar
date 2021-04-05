use chrono::Utc;
use futures::TryStreamExt;
use hmac::Mac;
use hyper::{Body, Method, Request, Response, StatusCode};
use std::clone::Clone;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::adapters::bucket::ListAllMyBucketsResult;
use crate::adapters::object::ListBucketResult;
use crate::adapters::user::OwnerResult;
use crate::drivers::s3::{Auth, Operation};
use crate::interactors::storage::Storage;

#[derive(Clone)]
pub struct App {
    pub storage: Arc<Mutex<Storage>>,
}

const AUTH_HEADER: &str = "Authorization";

impl App {
    pub async fn handle(self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let mut storage = self.storage.lock().await;
        let auth_str = req.headers().get(AUTH_HEADER).unwrap().to_str().unwrap();
        let auth = Auth::parse(auth_str);
        let user = storage.find_user(&auth.access_key).unwrap().to_owned();

        let string_to_sign = auth.string_to_sign(&req);
        let mut key = auth.key_builder(&user.secret_access_key);
        key.update(string_to_sign.as_bytes());

        if format!("{:x}", key.finalize().into_bytes()) != auth.signature {
            return Ok(Response::builder().status(401).body(Body::empty()).unwrap());
        }

        let result = match self.detect_operation(&req) {
            Operation::ListBuckets => Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(
                    ListAllMyBucketsResult {
                        buckets: storage
                            .list_buckets(&user.id)
                            .iter()
                            .map(|&b| b.into())
                            .collect(),
                        owner: OwnerResult {
                            display_name: user.display_name.to_string(),
                            id: user.id.to_string(),
                        },
                    }
                    .to_xml(),
                ))
                .unwrap(),
            Operation::CreateBucket(bucket) => {
                storage.create_bucket(&user.id, &bucket);
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
            }
            Operation::ListObjects(bucket) => Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(
                    ListBucketResult {
                        is_truncated: false,
                        contents: storage
                            .list_objects(&bucket)
                            .iter()
                            .map(|&o| o.into())
                            .collect(),
                        name: bucket,
                    }
                    .to_xml(),
                ))
                .unwrap(),
            Operation::PutObject(bucket, object) => {
                let entire_body = req
                    .into_body()
                    .try_fold(Vec::new(), |mut data, chunk| async move {
                        data.extend_from_slice(&chunk);
                        Ok(data)
                    })
                    .await
                    .unwrap();
                storage.put_object(&user, &bucket, &object, &entire_body);
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap()
            }
            Operation::GetObject(bucket, object) => {
                let mut buf = vec![];
                let obj = storage.get_object(&bucket, &object, &mut buf);

                Response::builder()
                    .status(StatusCode::OK)
                    .header(
                        "Last-Modified",
                        obj.last_modified
                            .with_timezone(&Utc)
                            .format("%a, %d %b %Y %H:%M:%S GMT")
                            .to_string(),
                    )
                    .body(Body::from(buf))
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
        let object = iter.next();

        match (req.method(), bucket, object) {
            (&Method::GET, Some(bucket), Some(object)) => {
                Operation::GetObject(bucket.to_string(), object.to_string())
            }
            (&Method::PUT, Some(bucket), Some(object)) => {
                Operation::PutObject(bucket.to_string(), object.to_string())
            }
            (&Method::PUT, Some(bucket), None) => Operation::CreateBucket(bucket.to_string()),
            (&Method::GET, Some(bucket), None) => Operation::ListObjects(bucket.to_string()),
            (_, _, _) => Operation::ListBuckets,
        }
    }
}
