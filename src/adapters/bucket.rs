use crate::adapters::user::OwnerResult;
use crate::entities::bucket::Bucket;

#[derive(Debug)]
pub struct BucketResult {
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
    pub fn to_xml(&self) -> String {
        format!("<Bucket><Name>{}</Name></Bucket>", self.name)
    }
}

#[derive(Debug)]
pub struct ListAllMyBucketsResult {
    pub buckets: Vec<BucketResult>,
    pub owner: OwnerResult,
}

impl ListAllMyBucketsResult {
    pub fn to_xml(&self) -> String {
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
