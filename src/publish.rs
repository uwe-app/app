use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use log::info;

use config::{AwsPublishEnvironment, ProfileSettings};
use publisher::{self, report::FileBuilder, PublishProvider, PublishRequest};

use workspace::{compile, Project};

use crate::{Error, Result};

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub env: String,
    pub provider: PublishProvider,
    pub exec: bool,
}

pub async fn publish(options: PublishOptions) -> Result<()> {
    let mut args = ProfileSettings::new_release();
    args.exec = Some(options.exec);
    let result = compile(&options.project, &args).await?;

    // FIXME: support multiple projects (workspaces)
    for project in result.projects.into_iter() {
        do_publish(&options, &project).await?;
    }

    Ok(())
}

async fn do_publish(options: &PublishOptions, project: &Project) -> Result<()> {
    match options.provider {
        PublishProvider::Aws => {
            if let Some(ref publish_config) =
                project.config.publish.as_ref().unwrap().aws
            {
                if let Some(env) = publish_config.environments.get(&options.env)
                {
                    let publish_env = env.clone();

                    let bucket = if let Some(ref bucket) = env.bucket {
                        bucket.to_string()
                    } else {
                        project.config.host.clone()
                    };

                    info!("Bucket {}", &bucket);

                    let region =
                        publisher::parse_region(&publish_config.region)?;

                    let request = PublishRequest {
                        region,
                        profile_name: publish_config.credentials.clone(),
                        bucket: bucket.clone(),
                        prefix: env.prefix.clone(),
                        keep_remote: env.keep_remote(),
                    };

                    publish_aws(project, request, &publish_env).await?
                } else {
                    return Err(Error::UnknownPublishEnvironment(
                        options.env.to_string(),
                    ));
                }
            } else {
                return Err(Error::NoPublishConfiguration);
            }
        }
    }

    Ok(())
}

async fn publish_aws(
    project: &Project,
    request: PublishRequest,
    env: &AwsPublishEnvironment,
) -> Result<()> {
    info!("Building local file list");

    // Create the list of local build files
    let mut file_builder =
        FileBuilder::new(project.options.base.clone(), env.prefix.clone());
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
