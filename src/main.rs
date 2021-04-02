#![feature(with_options)]
use std::fs::File;

use futures::TryStreamExt;
use hmac::{Hmac, Mac, NewMac};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::clone::Clone;
use std::convert::Infallible;
use std::fs;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug)]
struct User {
    id: String,
    display_name: String,
    access_key: String,
    secret_access_key: String,
}

#[derive(Clone, Debug)]
struct Object {
    name: String,
    bucket: String,
    owner_id: String,
    size: i64,
}

#[derive(Clone, Debug)]
struct Bucket {
    name: String,
    owner_id: String,
    object_count: i64,
    size: i64,
}

#[derive(Debug)]
struct BucketResult {
    name: String,
}

impl From<&Bucket> for BucketResult {
    fn from(bucket: &Bucket) -> BucketResult {
        Self {
            name: bucket.name.to_string(),
        }
    }
}

impl BucketResult {
    fn to_xml(&self) -> String {
        format!("<Bucket><Name>{}</Name></Bucket>", self.name)
    }
}

#[derive(Debug)]
struct OwnerResult {
    id: String,
    display_name: String,
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
    fn to_xml(&self) -> String {
        format!(
            "<Owner><DisplayName>{}</DisplayName><ID>{}</ID></Owner>",
            self.display_name, self.id
        )
    }
}

#[derive(Debug)]
struct ListAllMyBucketsResult {
    buckets: Vec<BucketResult>,
    owner: OwnerResult,
}

impl ListAllMyBucketsResult {
    fn to_xml(&self) -> String {
        format!(
            "<ListAllMyBucketsResult><Buckets>{}</Buckets>{}</ListAllMyBucketsResult>",
            self.buckets
                .iter()
                .map(|b| b.to_xml())
                .collect::<Vec<String>>()
                .join(""),
            self.owner.to_xml()
        )
    }
}

#[derive(Debug)]
struct ObjectResult {
    etag: String,
    key: String,
    owner: OwnerResult,
    size: i64,
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
        }
    }
}

impl ObjectResult {
    fn to_xml(&self) -> String {
        format!(
            "<Contents><ETag>{}</ETag><Key>{}</Key>{}<Size>{}</Size></Contents>",
            self.etag,
            self.key,
            self.owner.to_xml(),
            self.size
        )
    }
}

#[derive(Debug)]
struct ListBucketResult {
    is_truncated: bool,
    contents: Vec<ObjectResult>,
    name: String,
}

impl ListBucketResult {
    fn to_xml(&self) -> String {
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

#[derive(Clone)]
struct Storage {
    base_path: String,
    buckets: Vec<Bucket>,
    objects: Vec<Object>,
    users: Vec<User>,
}

impl Storage {
    fn new(base_path: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            buckets: vec![],
            objects: vec![],
            users: vec![],
        }
    }

    fn new_user(
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

    fn find_user(&self, id: &str) -> Option<&User> {
        self.users.iter().find(|&u| u.access_key == id)
    }

    fn create_bucket(&mut self, owner_id: &str, name: &str) {
        let path = Path::new(&self.base_path).join(name);

        if !path.exists() {
            fs::create_dir(path).unwrap();
        }

        self.buckets.push(Bucket {
            name: name.to_string(),
            owner_id: owner_id.to_string(),
            object_count: 0,
            size: 0,
        });
    }

    fn list_buckets(&self, owner_id: &str) -> ListAllMyBucketsResult {
        let user = self.users.iter().find(|&u| u.id == owner_id).unwrap();
        let buckets = self.buckets.iter().filter(|&b| b.owner_id == user.id);

        ListAllMyBucketsResult {
            buckets: buckets.map(|b| b.into()).collect(),
            owner: user.into(),
        }
    }

    fn list_objects(&self, bucket: &str) -> ListBucketResult {
        let objects = self.objects.iter().filter(|&o| o.bucket == bucket);

        ListBucketResult {
            is_truncated: false,
            contents: objects.map(|o| o.into()).collect(),
            name: bucket.to_string(),
        }
    }

    fn put_object(&mut self, user: &User, bucket: &str, object: &str, body: &[u8]) {
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
            name: object.to_string(),
            bucket: bucket.to_string(),
            owner_id: user.id.to_string(),
            size: body.len() as i64,
        })
    }

    fn get_object(&self, bucket: &str, object: &str, buf: &mut Vec<u8>) {
        let path = Path::new(&self.base_path).join(bucket).join(object);

        let mut file = File::open(path).unwrap();

        file.read_to_end(buf).unwrap();
    }
}

#[derive(Clone)]
struct App {
    storage: Arc<Mutex<Storage>>,
}

const AUTH_HEADER: &str = "Authorization";

#[derive(Debug)]
struct Auth {
    access_key: String,
    date: String,
    region: String,
    signature: String,
    signed_headers: Vec<String>,
}

impl Auth {
    fn parse(header: &str) -> Self {
        let re = Regex::new(
            r"^AWS4-HMAC-SHA256\sCredential=(?P<access_key>\w+)/(?P<date>\w+)/(?P<region>[\w-]+)/s3/aws4_request,\s*SignedHeaders=(?P<headers>[\w\-;]+),\s*Signature=(?P<signature>[0-9a-f]+)$",
        ).unwrap();

        let result = re.captures(header).unwrap();

        Self {
            access_key: result.name("access_key").unwrap().as_str().to_string(),
            date: result.name("date").unwrap().as_str().to_string(),
            region: result.name("region").unwrap().as_str().to_string(),
            signature: result.name("signature").unwrap().as_str().to_string(),
            signed_headers: result
                .name("headers")
                .unwrap()
                .as_str()
                .split(';')
                .map(|i| i.to_string())
                .collect(),
        }
    }

    fn key_builder(&self, secret_access_key: &str) -> HmacSha256 {
        let mut date_key =
            HmacSha256::new_varkey(format!("AWS4{}", secret_access_key).as_bytes()).unwrap();
        date_key.update(self.date.as_bytes());
        let mut region_key = HmacSha256::new_varkey(&date_key.finalize().into_bytes()).unwrap();
        region_key.update(self.region.as_bytes());
        let mut service_key = HmacSha256::new_varkey(&region_key.finalize().into_bytes()).unwrap();
        service_key.update(b"s3");
        let mut signing_key = HmacSha256::new_varkey(&service_key.finalize().into_bytes()).unwrap();
        signing_key.update(b"aws4_request");

        HmacSha256::new_varkey(&signing_key.finalize().into_bytes()).unwrap()
    }

    fn canonical_request(&self, req: &Request<Body>) -> String {
        [
            req.method().as_str(),
            req.uri().path(),
            req.uri().query().unwrap_or(""),
            &self
                .signed_headers
                .iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        key,
                        req.headers().get(key).unwrap().to_str().unwrap().trim()
                    )
                })
                .collect::<Vec<String>>()
                .join("\n"),
            "",
            &self.signed_headers.join(";"),
            req.headers()
                .get("x-amz-content-sha256")
                .unwrap()
                .to_str()
                .unwrap(),
        ]
        .join("\n")
    }

    fn string_to_sign(&self, req: &Request<Body>) -> String {
        let mut hash = Sha256::default();
        hash.update(&self.canonical_request(req));
        format!(
            "AWS4-HMAC-SHA256\n{}\n{}/{}/s3/aws4_request\n{:x}",
            req.headers().get("x-amz-date").unwrap().to_str().unwrap(),
            &self.date,
            &self.region,
            hash.finalize()
        )
    }
}

enum Operation {
    ListBuckets,
    ListObjects(String),
    CreateBucket(String),
    GetObject(String, String),
    PutObject(String, String),
}

impl App {
    async fn handle(self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
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
            Operation::ListBuckets => storage.list_buckets(&user.id).to_xml(),
            Operation::CreateBucket(bucket) => {
                storage.create_bucket(&user.id, &bucket);
                "".to_string()
            }
            Operation::ListObjects(bucket) => storage.list_objects(&bucket).to_xml(),
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
                "".to_string()
            }
            Operation::GetObject(bucket, object) => {
                let mut buf = vec![];
                storage.get_object(&bucket, &object, &mut buf);

                String::from_utf8(buf).unwrap()
            }
        };

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Last-Modified", "Wed, 12 Oct 2009 17:50:00 GMT") // TODO: use legit value instead
            .body(Body::from(result))
            .unwrap())
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

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let mut s = Storage::new("/home/mehdy/tmp/anbar");
    s.new_user("mehdy", "Mehdy", "ABC1234", "AbC1Zxv");
    s.create_bucket("mehdy", "buck");

    let storage = Arc::new(Mutex::new(s));
    let service = make_service_fn(move |_conn| {
        let app = App {
            storage: storage.clone(),
        };
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let app = app.clone();
                app.handle(req)
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
