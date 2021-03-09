use project::ProjectOptions;
use crate::Result;

pub async fn project(options: ProjectOptions) -> Result<()> {
    Ok(project::create(options).await?)
}

