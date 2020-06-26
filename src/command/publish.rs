use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;

use rusoto_core::Region;

use log::info;

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

    // Compile a pristine release
    let mut args: BuildArguments = Default::default();
    args.release = Some(true);
    let ctx = workspace::compile_from(&mut config, &args)?;

    let publish = config.publish.as_ref().unwrap();

    match options.provider {
        PublishProvider::Aws => {

            if let Some(ref publish_config) = publish.aws {

                let path = get_aws_path(publish_config, &options.path)?;

                let prefix = if path.is_empty() {
                    None
                } else {
                    Some(path.clone())
                };

                info!("Building local file list...");

                // Create the list of local build files
                let mut file_builder = FileBuilder::new(ctx.options.base.clone(), prefix.clone());
                file_builder.walk()?;

                info!("Local objects {}", file_builder.keys.len());

                //println!("Got builder files {:?}", file_builder.paths);
                //std::process::exit(1);

                let region = Region::from_str(&publish_config.region)?;

                let request = PublishRequest {
                    profile_name: publish_config.credentials.clone(),
                    region,
                    bucket: publish_config.bucket.as_ref().unwrap().clone(),
                    prefix,
                };

                info!("Building remote file list...");

                //let local = &file_builder.paths;
                let mut remote: HashSet<String> = HashSet::new();
                let mut etags: HashMap<String, String> = HashMap::new();
                publisher::list_remote(&request, &mut remote, &mut etags).await?;

                //println!("Got local list {:?}", file_builder.paths);
                //println!("Got remote list {:?}", remote);

                info!("Remote objects {}", remote.len());

                let diff = publisher::diff(&file_builder, &remote, &etags)?;

                let push: HashSet<_> = diff.upload.union(&diff.changed).collect();
                for k in push {
                    info!("Upload {}", k);
                }

                for k in &diff.deleted {
                    info!("Delete {}", k);
                }

                //info!("Ok (up to date) {}", diff.same.len());
                info!("New {}", diff.upload.len());
                info!("Update {}", diff.changed.len());
                info!("Delete {}", diff.deleted.len());

                //publisher::publish(&request).await?;
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
