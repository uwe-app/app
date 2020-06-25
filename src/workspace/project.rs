use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::{debug, info};

use crate::{Error, Result};
use crate::command::build::{BuildOptions, BuildTag};
use crate::config::{BuildArguments, Config};
use crate::{utils, LAYOUT_HBS};

use super::Workspace;

fn with(cfg: &mut Config, args: &BuildArguments) -> Result<BuildOptions> {
    let build = cfg.build.as_ref().unwrap();
    let release = args.release.is_some() && args.release.unwrap();

    let (tag_target, target_dir) = get_tag_info(args);

    let mut target = build.target.clone();
    if !target_dir.is_empty() {
        let target_dir_buf = PathBuf::from(&target_dir);
        if target_dir_buf.is_absolute() {
            return Err(Error::new(format!(
                "Build tag may not be an absolute path {}",
                target_dir
            )));
        }
        target.push(target_dir);
    }

    let live = args.live.is_some() && args.live.unwrap();
    let mut force = args.force.is_some() && args.force.unwrap();
    let include_index = args.include_index.is_some() && args.include_index.unwrap();

    if live && release {
        return Err(Error::new(
            "Live reload is not available for release builds".to_string(),
        ));
    }

    if include_index {
        let link = cfg.link.as_mut().unwrap();
        if let Some(ref mut include_index) = link.include_index {
            *include_index = true;
        }
    }

    let incremental = args.incremental.is_some() && args.incremental.unwrap();
    let pristine = args.pristine.is_some() && args.pristine.unwrap();

    if (pristine || force) && target.exists() {
        info!("clean {}", target.display());
        fs::remove_dir_all(&target)?;
    }

    // Force is implied when live and incremental, the live
    // setting overrides the incremental behavior
    if live && incremental && !force {
       force = true;  
    }

    utils::require_output_dir(&target)?;

    let serve = cfg.serve.as_ref().unwrap();
    let mut host = &serve.host;
    let mut port = &serve.port;

    if let Some(h) = &args.host {
        host = h;
    }

    if let Some(p) = &args.port {
        port = p;
    }

    if !build.source.exists() {
        return Err(Error::new(format!(
            "Source directory does not exist {}",
            build.source.display()
        )));
    }

    let mut layout = build.source.clone();
    if let Some(ref custom_layout) = args.layout {
        layout.push(custom_layout);
    } else {
        layout.push(LAYOUT_HBS);
    };

    if !layout.exists() {
        return Err(Error::new(format!(
            "Missing layout file '{}'",
            layout.display()
        )));
    }

    let clean_url = build.clean_url.is_some() && build.clean_url.unwrap();

    let opts = BuildOptions {
        source: build.source.clone(),
        output: build.target.clone(),
        base: target.clone(),
        host: host.to_owned(),
        port: port.to_owned(),

        clean_url,
        target,
        layout,
        max_depth: args.max_depth,
        release,
        live,
        force,
        tag: tag_target,
        paths: args.paths.clone(),
        base_href: args.base.clone(),
        incremental,
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

// Map a set of paths making them relative to the source, used when
// paths are defined in the `paths` definition of a profile in the configuration.
//
// When we get paths from the command line there is no need to prefix them.
fn prefix(source: &PathBuf, paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|p| {
            let mut pth = source.clone();
            pth.push(p);
            pth
        })
        .collect::<Vec<_>>()
}

pub fn prepare(cfg: &mut Config, args: &BuildArguments) -> Result<BuildOptions> {
    let (_, target_dir) = get_tag_info(args);

    // Handle profiles, eg: [profile.dist] that mutate the
    // arguments from config declarations
    let profiles = cfg.profile.as_mut().unwrap();
    if let Some(ref mut profile) = profiles.get_mut(&target_dir) {
        if profile.tag.is_some() {
            return Err(Error::new(format!(
                "Profiles may not define a build tag, please remove it"
            )));
        }

        if let Some(ref paths) = profile.paths {
            let build = cfg.build.as_ref().unwrap();
            let paths = prefix(&build.source, paths);
            debug!("profile paths {:?}", &paths);
            profile.paths = Some(paths);
        }

        let merged = utils::merge::map::<BuildArguments>(profile, args)?;

        // Always update base to use the path separator. The declaration is
        // a URL path but internally we treat as a filesystem path.
        if let Some(ref base) = merged.base {
            profile.base = Some(utils::url::to_path_separator(base));
        }

        return with(cfg, &merged);
    }

    with(cfg, args)
}
