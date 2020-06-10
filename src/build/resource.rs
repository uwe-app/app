use std::io::ErrorKind;
use std::path::Path;
use std::convert::AsRef;

use crate::Error;
use super::context::Context;

use log::{info, debug};

#[cfg(windows)]
fn symlink<P: AsRef<Path>>(source: P, target: P) -> Result<(), Error> {
    let path = source.as_ref();
    if path.is_dir() {
        return std::os::windows::fs::symlink_dir(source, target)
            .map_err(Error::from);
    } else if path.is_file() {
        return std::os::windows::fs::symlink_file(source, target)
            .map_err(Error::from);
    }
    Ok(())
}

#[cfg(unix)]
fn symlink<P: AsRef<Path>>(source: P, target: P) -> Result<(), Error> {
    std::os::unix::fs::symlink(source, target).map_err(Error::from)
}

pub fn link(ctx: &Context) -> Result<(), Error> {
    let target = &ctx.options.target;

    // The resource path must be absolute for links to work
    // regardless of where the executable is run from
    let result = ctx.config.get_resources_path(&ctx.options.source).canonicalize();

    match result {
        Ok(resource) => {
            if resource.exists() {
                if resource.is_dir() {
                    let result = resource.read_dir()?;
                    for res in result {
                        let entry = res?;
                        let path = entry.path();
                        if let Some(name) = path.file_name() {
                            let mut dest = target.clone();
                            dest.push(name);

                            if dest.exists() {
                                debug!("symlink exists ({} -> {})", path.display(), dest.display());
                                continue;
                            }

                            info!("ln -s {} -> {}", path.display(), dest.display());
                            symlink(&path, &dest)?;
                        }
                    }

                } else {
                    return Err(Error::new("Resources must be a directory".to_string()));
                }
            }
        },
        Err(e) => {
            match e.kind() {
                 ErrorKind::NotFound => {
                    // It is fine for the resource directory not to exist
                    // as we set a default value and it may not be in use
                 },
                 _ => {
                    return Err(Error::from(e))
                 }
            }
        }
    }

    Ok(())
}
