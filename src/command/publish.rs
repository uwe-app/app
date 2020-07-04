use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use log::info;

use config::{BuildArguments, Config};
use publisher::{self, PublishProvider, PublishRequest};
use report::FileBuilder;

use workspace;

use crate::Error;
use crate::Result;

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub env: String,
    pub provider: PublishProvider,
}

pub fn publish(options: PublishOptions) -> Result<()> {
    let mut spaces: Vec<Config> = Vec::new();
    workspace::find(&options.project, true, &mut spaces)?;
    for space in spaces {
        publish_one(&options, &space)?;
    }
    Ok(())
}

#[tokio::main]
async fn publish_one(options: &PublishOptions, config: &Config) -> Result<()> {
    match options.provider {
        PublishProvider::Aws => {
            if let Some(ref publish_config) = config.publish.as_ref().unwrap().aws {
                if let Some(ref env) = publish_config.environments.get(&options.env) {
                    let bucket = if let Some(ref bucket) = env.bucket {
                        bucket.to_string()
                    } else {
                        config.host.clone()
                    };

                    info!("Bucket {}", &bucket);

                    let region = publisher::parse_region(&publish_config.region)?;

                    let request = PublishRequest {
                        region,
                        profile_name: publish_config.credentials.clone(),
                        bucket: bucket.clone(),
                        prefix: env.prefix.clone(),
                    };

                    // Compile a pristine release
                    let mut args: BuildArguments = Default::default();
                    args.release = Some(true);
                    let ctx = workspace::compile(&config, &args, false)?;

                    info!("Building local file list");

                    // Create the list of local build files
                    let mut file_builder =
                        FileBuilder::new(ctx.options.base.clone(), env.prefix.clone());
                    file_builder.walk()?;

                    info!("Local objects {}", file_builder.keys.len());

                    info!("Building remote file list");

                    let mut remote: HashSet<String> = HashSet::new();
                    let mut etags: HashMap<String, String> = HashMap::new();
                    publisher::list_remote(&request, &mut remote, &mut etags).await?;

                    info!("Remote objects {}", remote.len());

                    let diff = publisher::diff(&file_builder, &remote, &etags)?;
                    publisher::publish(&request, file_builder, diff).await?;
                } else {
                    return Err(Error::UnknownPublishEnvironment(options.env.to_string()));
                }
            } else {
                return Err(Error::NoPublishConfiguration);
            }
        }
    }

    Ok(())
}
