use std::path::Path;

use crate::{Error, ErrorCallback};
use config::ProfileSettings;

mod invalidator;
mod livereload;

pub async fn compile<P: AsRef<Path>>(
    project: P,
    args: &mut ProfileSettings,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let live = args.live.is_some() && args.live.unwrap();
    if live {
        livereload::start(project, args, error_cb).await?;
    } else {
        workspace::compile_project(project, args).await?;
    }
    Ok(())
}
