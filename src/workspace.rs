use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::{info, debug};

use crate::config::{Config, BuildArguments};
use crate::{utils, Error, LAYOUT_HBS};
use crate::command::build::{BuildTag, BuildOptions};

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

fn with(cfg: &mut Config, args: &BuildArguments) -> Result<BuildOptions, Error> {

    let build = cfg.build.as_ref().unwrap();
    let release = args.release.is_some() && args.release.unwrap();

    let (tag_target, target_dir) = get_tag_info(args);

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

    let live = args.live.is_some() && args.live.unwrap();
    let force = args.force.is_some() && args.force.unwrap();
    let include_index = args.include_index.is_some() && args.include_index.unwrap();

    if live && release {
        return Err(
            Error::new(
                "Live reload is not available for release builds".to_string()))
    }

    if include_index {
        let link = cfg.link.as_mut().unwrap();
        if let Some(ref mut include_index) = link.include_index {
            *include_index = true;
        }
    }

    if force && target.exists() {
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

    let mut layout = build.source.clone();
    if let Some(ref custom_layout) = args.layout {
        layout.push(custom_layout);
    } else {
        layout.push(LAYOUT_HBS);
    };

    if !layout.exists() {
        return Err(
            Error::new(
                format!("Missing layout file '{}'", layout.display())));
    }

    let clean_url = build.clean_url.is_some()
        && build.clean_url.unwrap();

    let opts = BuildOptions {
        source: build.source.clone(),
        output: build.target.clone(),
        base: target.clone(),
        host: host.to_owned(),
        port: port.to_owned(),

        clean_url,
        target,
        from,
        layout,
        directory: dir,
        max_depth: args.max_depth,
        release: release,
        live: live,
        force: force,
        tag: tag_target,
        copy: args.copy.clone(),
    };

    debug!("{:?}", &cfg);

    Ok(opts)
}

fn get_tag_info(args: &BuildArguments) -> (BuildTag, String) {
    let release = args.release.is_some() && args.release.unwrap();

    let mut tag_target = BuildTag::Debug;
    if release {
        tag_target = BuildTag::Release;
    }

    if let Some(t) = &args.tag {
        if !t.is_empty() {
            tag_target = BuildTag::Custom(t.to_string());
        }
    }

    let target_dir = tag_target.get_path_name();
    (tag_target, target_dir)
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

pub fn prepare(cfg: &mut Config, args: &BuildArguments) -> Result<BuildOptions, Error> {
    let (_, target_dir) = get_tag_info(args);

    // Handle profiles, eg: [profile.dist] that mutate the 
    // arguments from config declarations
    let profiles = cfg.profile.as_ref().unwrap();
    if let Some(ref profile) = profiles.get(&target_dir) {

        if profile.tag.is_some() {
            return Err(
                Error::new(
                    format!("Profiles may not define a build tag, please remove it")));
        }

        let merged = utils::merge::map::<BuildArguments>(profile, args)?;
        return with(cfg, &merged);
    }

    with(cfg, args)
}
