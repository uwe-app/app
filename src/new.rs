use crate::Result;
use project::ProjectOptions;

pub async fn project(options: ProjectOptions) -> Result<()> {
    project::create(options).await?;
    Ok(())
}
