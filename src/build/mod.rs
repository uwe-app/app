use std::path::Path;

use crate::{Error, ErrorCallback};
use config::{lock_file::LockFile, ProfileSettings};

use scopeguard::defer;
use workspace::lock;

mod livereload;

pub async fn compile<P: AsRef<Path>>(
    project: P,
    args: &'static mut ProfileSettings,
    error_cb: ErrorCallback,
) -> Result<(), Error> {
    let lock_path = LockFile::get_lock_file(project.as_ref());
    let lock_file = lock::acquire(&lock_path)?;
    defer! { let _ = lock::release(lock_file); }

    let live = args.live.is_some() && args.live.unwrap();
    if live {
        let args = Box::leak(Box::new(args));
        livereload::start(project, args, error_cb).await?;
    } else {
        workspace::compile(project, args).await?;
    }

    Ok(())
}
