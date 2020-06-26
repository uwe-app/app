extern crate futures;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate tokio_core;

use rusoto_core::request::HttpClient;
use rusoto_core::credential;
use rusoto_core::Region;
use rusoto_s3::{
    S3Client, S3, ListObjectsV2Request, ListObjectsV2Output, ListObjectsV2Error, HeadBucketRequest};

use crate::AwsResult;

use futures::future::Future;
use tokio_core::reactor::Core;

#[derive(Debug)]
pub struct PublishRequest {
    pub profile_name: String,
    pub region: Region,
    pub bucket: String, 
    pub path: String,
}

#[derive(Debug)]
pub enum PublishProvider {
    Aws,
}

fn get_client(request: &PublishRequest) -> AwsResult<S3Client> {
    let mut provider = credential::ProfileProvider::new()?;
    provider.set_profile(&request.profile_name);
    let dispatcher = HttpClient::new()?;
    let client = S3Client::new_with(dispatcher, provider, request.region.clone());
    Ok(client)
}

pub async fn list_bucket_keys(client: &S3Client, request: &PublishRequest) -> impl Future<Item = ListObjectsV2Output, Error = ListObjectsV2Error> {

    println!("Publisher request {:?}", request);

    //let client = get_client(request)?;

    let req = ListObjectsV2Request {
        bucket: request.bucket.clone(),
        ..Default::default()
    };

    client.list_objects_v2(req)
}

//fn head_bucket(request: &PublishRequest, client: &S3Client) -> impl Future<()> {
    ////let client = get_client(request)?;
    //let req = HeadBucketRequest {
        //bucket: request.bucket.clone(),
    //};
    //client.head_bucket(req)
//}

pub async fn publish(request: &PublishRequest) -> AwsResult<()> {

    println!("Publisher head request {:?}", request);

    let client = get_client(request)?;

    //let head = head_bucket(&request, &client);

    //let req = HeadBucketRequest {
        //bucket: request.bucket.clone(),
    //};

    //let result = client.head_bucket(req);
        //.await?;
    //println!("Publisher result {:?}", result);

    Ok(())
}
