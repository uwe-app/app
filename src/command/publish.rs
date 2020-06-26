use std::path::PathBuf;

use rusoto_core::Region;
use std::str::FromStr;

use crate::Error;
use crate::Result;
use crate::build::report::FileBuilder;
use crate::config::{Config, AwsPublishConfig, BuildArguments};
use crate::workspace::{self, Workspace};

use crate::publisher::{self, PublishRequest, PublishProvider};

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub path: Option<String>,
    pub provider: PublishProvider,
}

pub fn publish(options: PublishOptions) -> Result<()> {
    let mut spaces: Vec<Workspace> = Vec::new();
    workspace::find(&options.project, true, &mut spaces)?;
    for mut space in spaces {
        publish_one(&options, &mut space.config)?;
    }
    Ok(())
}

#[tokio::main]
async fn publish_one(options: &PublishOptions, mut config: &mut Config) -> Result<()> {

    let publish = config.publish.as_ref().unwrap();

    match options.provider {
        PublishProvider::Aws => {

            if let Some(ref publish_config) = publish.aws {

                let path = get_aws_path(publish_config, &options.path)?;

                let prefix = if path.is_empty() {
                    None
                } else {
                    Some(path)
                };

                // Compile a pristine release
                let mut args: BuildArguments = Default::default();
                args.release = Some(true);
                let ctx = workspace::compile_from(&mut config, &args)?;

                // Create the list of local build files
                let mut file_builder = FileBuilder::new(ctx.options.base.clone(), prefix);
                file_builder.walk()?;

                println!("Got builder files {:?}", file_builder.paths);

                std::process::exit(1);


                let region = Region::from_str(&publish_config.region)?;

                let request = PublishRequest {
                    profile_name: publish_config.credentials.clone(),
                    region,
                    bucket: publish_config.bucket.as_ref().unwrap().clone(),
                    path,
                };

                println!("Trying to publish");

                publisher::publish(&request).await?;
            } else {
                return Err(Error::new(format!("No publish configuration")))
            }

        },
    }

    Ok(())
}

fn get_aws_path<'a>(config: &'a AwsPublishConfig, req_path: &Option<String>) -> Result<String> {
    let mut path = String::from("");

    if let Some(ref target_path) = req_path {
        path = find_aws_path(config, target_path);
        if !target_path.is_empty() && path.is_empty() {
            return Err(
                Error::new(
                    format!("Unknown remote path '{}', check the publish configuration", target_path)))
        }
    }

    Ok(path)
}

fn find_aws_path<'a>(config: &'a AwsPublishConfig, name: &str) -> String {
    if config.paths.contains_key(name) {
        return config.paths.get(name).unwrap().to_string();
    }
    String::from("")
}
