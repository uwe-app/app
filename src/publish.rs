use std::path::PathBuf;

use config::ProfileSettings;
use publisher::{self, PublishProvider, AwsPublishRequest, aws_publish};

use workspace::{compile, Project};

use crate::{Error, Result};

#[derive(Debug)]
pub struct PublishOptions {
    pub project: PathBuf,
    pub env: String,
    pub provider: PublishProvider,
    pub exec: bool,
    pub sync_redirects: bool,
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
                    //let publish_env = env.clone();

                    let bucket = if let Some(ref bucket) = env.bucket {
                        bucket.to_string()
                    } else {
                        project.config.host.clone()
                    };

                    let region =
                        publisher::parse_region(&publish_config.region)?;

                    let request = AwsPublishRequest {
                        region,
                        profile_name: publish_config.credentials.clone(),
                        bucket: bucket.clone(),
                        prefix: env.prefix.clone(),
                        keep_remote: env.keep_remote(),
                        build_target: project.options.build_target().clone(),
                        sync_redirects: options.sync_redirects,
                        redirects_manifest: None,
                    };

                    aws_publish(request).await?
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
