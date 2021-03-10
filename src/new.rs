use project::ProjectOptions;
use crate::Result;

pub async fn project(options: ProjectOptions) -> Result<()> {
    project::create(options).await?;
    Ok(())
}

