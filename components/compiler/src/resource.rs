use std::io::ErrorKind;

use log::{debug, info};

use utils::symlink;

use super::context::Context;
use crate::Error;
use crate::Result;

pub fn link(ctx: &Context) -> Result<()> {
    let target = &ctx.options.target;

    // The resource path must be absolute for links to work
    // regardless of where the executable is run from
    let result = ctx
        .config
        .get_resources_path(&ctx.options.source)
        .canonicalize();

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
                            symlink::soft(&path, &dest)?;
                        }
                    }
                } else {
                    return Err(Error::new("Resources must be a directory".to_string()));
                }
            }
        }
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => {
                    // It is fine for the resource directory not to exist
                    // as we set a default value and it may not be in use
                }
                _ => return Err(Error::from(e)),
            }
        }
    }

    Ok(())
}
