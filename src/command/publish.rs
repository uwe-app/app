use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use log::info;

use compiler::BuildContext;
use config::AwsPublishEnvironment;
use config::{Config, ProfileSettings};
use publisher::{self, PublishProvider, PublishRequest};
use report::FileBuilder;

use workspace::lock;
use scopeguard::defer;

use crate::Error;
use crate::Result;

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub env: String,
    pub provider: PublishProvider,
}

pub async fn publish(options: PublishOptions) -> Result<()> {

    let lock_path = options.project.join("site.lock");
    let lock_file = lock::acquire(&lock_path)?;
    defer! { let _ = lock::release(lock_file); }

    let mut spaces = workspace::open(&options.project, true)?;
    for entry in spaces.iter_mut() {
        build_publish(&options, &mut entry.config).await?;
    }
    Ok(())
}

async fn build_publish(options: &PublishOptions, config: &mut Config) -> Result<()> {
    match options.provider {
        PublishProvider::Aws => {
            if let Some(ref publish_config) = config.publish.as_ref().unwrap().aws {
                if let Some(env) = publish_config.environments.get(&options.env) {
                    let publish_env = env.clone();

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
                    let mut args: ProfileSettings = Default::default();
                    args.release = Some(true);
                    let (ctx, _locales) = workspace::compile1(config, &mut args).await?;

                    publish_aws(ctx, request, &publish_env).await?
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

async fn publish_aws(
    ctx: BuildContext,
    request: PublishRequest,
    env: &AwsPublishEnvironment,
) -> Result<()> {
    info!("Building local file list");

    // Create the list of local build files
    let mut file_builder = FileBuilder::new(ctx.options.base.clone(), env.prefix.clone());
    file_builder.walk()?;

    info!("Local objects {}", file_builder.keys.len());

    info!("Building remote file list");

    let mut remote: HashSet<String> = HashSet::new();
    let mut etags: HashMap<String, String> = HashMap::new();
    publisher::list_remote(&request, &mut remote, &mut etags).await?;

    info!("Remote objects {}", remote.len());

    let diff = publisher::diff(&file_builder, &remote, &etags)?;
    publisher::publish(&request, file_builder, diff).await?;

    Ok(())
}
