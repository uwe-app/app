use std::path::Path;
use std::io::ErrorKind;

use ignore::WalkBuilder;

use crate::Result;
use crate::Error;
use super::context::Context;
use crate::utils::symlink;
use crate::build::BuildFiles;

use log::{debug, info};

fn add_to_build<P: AsRef<Path>>(ctx: &Context, resource: P, path: P, build_files: &mut BuildFiles) -> Result<()> {
    for result in WalkBuilder::new(path.as_ref())
        .follow_links(true)
        .build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    // Need to make the path look like an output destination
                    let relative = path.strip_prefix(resource.as_ref())?;
                    let mut base = ctx.options.base.clone();
                    base.push(relative);

                    build_files.add(&base, None)?;
                }
            }
            Err(e) => return Err(Error::from(e)),
        }
    }
    Ok(())
}

pub fn link(ctx: &Context, build_files: &mut Option<BuildFiles>) -> Result<()> {
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

                        if let Some(mut build_files) = build_files.as_mut() {
                            if path.is_file() {
                                build_files.add(&path, None)?;
                            } else {
                                add_to_build(ctx, &resource, &path, &mut build_files)?;
                            }
                        }

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
