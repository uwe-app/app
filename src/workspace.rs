use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::{info, debug};

use crate::config::Config;
use crate::Error;
use crate::command::build::{BuildTag, BuildOptions, BuildArguments};

pub struct Workspace {
    pub config: Config
}

impl Workspace {
    pub fn new(config: Config) -> Self {
        Self {config} 
    }
}

fn create_output_dir(output: &PathBuf) -> Result<(), Error> {
    if !output.exists() {
        info!("mkdir {}", output.display());
        fs::create_dir_all(output)?;
    }

    if !output.is_dir() {
        return Err(
            Error::new(
                format!("Not a directory: {}", output.display())));
    }

    Ok(())
}

pub fn prepare(cfg: &Config, args: &BuildArguments) -> Result<BuildOptions, Error> {

    if args.live && args.release {
        return Err(
            Error::new(
                "Live reload is not available for release builds".to_string()))
    }

    let build = cfg.build.as_ref().unwrap();

    let mut tag_target = BuildTag::Debug;
    if args.release {
        tag_target = BuildTag::Release;
    }

    if let Some(t) = &args.tag {
        if !t.is_empty() {
            tag_target = BuildTag::Custom(t.to_string());
        }
    }

    let target_dir = tag_target.get_path_name();
    info!("{}", target_dir);

    let mut target = build.target.clone();
    if !target_dir.is_empty() {
        let target_dir_buf = PathBuf::from(&target_dir);

        if target_dir_buf.is_absolute() {
            return Err(
                Error::new(
                    format!("Build tag may not be an absolute path {}", target_dir)));
        }

        target.push(target_dir);
    }

    if args.force && target.exists() {
        info!("rm -rf {}", target.display());
        fs::remove_dir_all(&target)?;
    }

    create_output_dir(&target)?;

    let mut dir = None;
    if let Some(d) = &args.directory {
        if d.is_absolute() {
            return Err(
                Error::new(
                    format!("Directory must be relative {}", d.display())));
        }
        let mut src = build.source.clone();
        src.push(d);
        if !src.exists() {
            return Err(
                Error::new(
                    format!("Target directory does not exist {}", src.display())));
        }
        dir = Some(src);
    }

    let serve = cfg.serve.as_ref().unwrap();
    let mut host = &serve.host;
    let mut port = &serve.port;

    if let Some(h) = &args.host {
        host = h;
    }

    if let Some(p) = &args.port {
        port = p;
    }

    let mut from = build.source.clone();
    if let Some(dir) = &dir {
        from = dir.clone().to_path_buf();
    }

    let clean_url = build.clean_url.is_some()
        && build.clean_url.unwrap();

    let opts = BuildOptions {
        source: build.source.clone(),
        output: build.target.clone(),
        host: host.to_owned(),
        port: port.to_owned(),

        clean_url,
        target,
        from,
        directory: dir,
        max_depth: args.max_depth,
        release: args.release,
        live: args.live,
        force: args.force,
        index_links: args.index_links,
        tag: tag_target,
    };

    debug!("{:?}", &cfg);

    Ok(opts)
}

pub fn load<P: AsRef<Path>>(dir: P, walk_ancestors: bool, spaces: &mut Vec<Workspace>) -> Result<(), Error> {

    let project = dir.as_ref();
    let cfg = Config::load(&project, walk_ancestors)?;

    if let Some(ref workspaces) = &cfg.workspace {
        for space in &workspaces.members {
            let mut root = cfg.get_project();
            root.push(space);
            if !root.exists() || root.is_file() {
                return Err(
                    Error::new(
                        format!("Workspace must be a directory")));
            }

            // Recursive so that workspaces can reference
            // other workspaces if they need to
            load(root, false, spaces)?;
        }
    } else {
        spaces.push(Workspace::new(cfg)); 
    }

    Ok(())
}
