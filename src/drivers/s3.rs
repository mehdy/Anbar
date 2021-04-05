use hmac::{Hmac, Mac, NewMac};
use hyper::{Body, Request};
use regex::Regex;
use sha2::{Digest, Sha256};

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub struct Auth {
    pub access_key: String,
    pub date: String,
    pub region: String,
    pub signature: String,
    pub signed_headers: Vec<String>,
}

pub enum Operation {
    ListBuckets,
    ListObjects(String),
    CreateBucket(String),
    GetObject(String, String),
    PutObject(String, String),
}

impl Auth {
    pub fn parse(header: &str) -> Self {
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

    pub fn key_builder(&self, secret_access_key: &str) -> HmacSha256 {
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

    pub fn canonical_request(&self, req: &Request<Body>) -> String {
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

    pub fn string_to_sign(&self, req: &Request<Body>) -> String {
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
