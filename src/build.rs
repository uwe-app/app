use std::path::Path;

use crate::Error;
use config::ProfileSettings;

pub async fn compile<P: AsRef<Path>>(
    project: P,
    args: ProfileSettings,
) -> Result<(), Error> {
    workspace::compile(project, &args, Default::default(), false).await?;
    Ok(())
}
