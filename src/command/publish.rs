use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;

use rusoto_core::Region;

use log::info;

use crate::Error;
use crate::Result;
use crate::build::report::FileBuilder;
use crate::config::{Config, BuildArguments};
use crate::workspace::{self, Workspace};

use crate::publisher::{self, PublishRequest, PublishProvider};

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub env: Option<String>,
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
    match options.provider {
        PublishProvider::Aws => {
            if let Some(ref publish_config) = config.publish.as_mut().unwrap().aws {

                let mut env = Default::default();
                if let Some(ref env_name) = options.env {
                    env = publish_config.environments.get(env_name);
                    if env.is_none() {
                        return Err(
                            Error::new(
                                format!(
                                    "Unknown publish environment '{}'", env_name)))
                    }
                }

                let mut prefix = None;
                if let Some(environ) = env {
                    prefix = if !environ.prefix.is_empty() {
                        Some(environ.prefix.clone())
                    } else {
                        None
                    };
                }

                let region = Region::from_str(&publish_config.region)?;

                let request = PublishRequest {
                    region,
                    profile_name: publish_config.credentials.clone(),
                    bucket: publish_config.bucket.as_ref().unwrap().clone(),
                    prefix: prefix.clone(),
                };

                // Compile a pristine release
                let mut args: BuildArguments = Default::default();
                args.release = Some(true);
                let ctx = workspace::compile_from(&mut config, &args)?;

                info!("Building local file list");

                // Create the list of local build files
                let mut file_builder = FileBuilder::new(ctx.options.base.clone(), prefix.clone());
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
                return Err(Error::new(format!("No publish configuration")))
            }

        },
    }

    Ok(())
}
