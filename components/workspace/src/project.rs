use std::fs;
use std::path::PathBuf;

use log::{debug, info};

use config::{ProfileName, RuntimeOptions};
use compiler::redirect;
use config::{ProfileSettings, Config};
use utils;

use crate::{Error, Result};

static LAYOUT_HBS: &str = "layout.hbs";

fn require_output_dir(output: &PathBuf) -> Result<()> {
    if !output.exists() {
        info!("mkdir {}", output.display());
        std::fs::create_dir_all(output)?;
    }

    if !output.is_dir() {
        return Err(Error::NotDirectory(output.clone()));
    }

    Ok(())
}

fn with(cfg: &Config, args: &ProfileSettings) -> Result<RuntimeOptions> {
    let build = cfg.build.as_ref().unwrap();
    let release = args.release.is_some() && args.release.unwrap();

    let (tag_target, target_dir) = get_tag_info(args);

    let mut target = build.target.clone();
    if !target_dir.is_empty() {
        let target_dir_buf = PathBuf::from(&target_dir);
        if target_dir_buf.is_absolute() {
            return Err(Error::ProfileNameAbsolute(target_dir));
        }
        target.push(target_dir);
    }

    let live = args.live.is_some() && args.live.unwrap();
    let mut force = args.force.is_some() && args.force.unwrap();

    let include_index = args.include_index.is_some() && args.include_index.unwrap();

    if live && release {
        return Err(Error::LiveReloadRelease);
    }

    if let Some(ref redirects) = cfg.redirect {
        redirect::validate(redirects)?;
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

    require_output_dir(&target)?;

    let serve = cfg.serve.as_ref().unwrap();
    let mut host = &serve.host;
    let mut port = &serve.port;

    if let Some(h) = &args.host {
        host = h;
    }

    if let Some(p) = &args.port {
        port = p;
    }

    if !build.source.exists() || !build.source.is_dir() {
        return Err(Error::NotDirectory(build.source.clone()));
    }

    let mut layout = build.source.clone();
    if let Some(ref custom_layout) = args.layout {
        layout.push(custom_layout);
    } else {
        layout.push(LAYOUT_HBS);
    };

    let use_layout = build.use_layout.is_some() && build.use_layout.unwrap();
    if use_layout && !layout.exists() {
        return Err(Error::NoLayout(layout));
    }

    let rewrite_index = build.rewrite_index.is_some() && build.rewrite_index.unwrap();

    let opts = RuntimeOptions {
        source: build.source.clone(),
        output: build.target.clone(),
        base: target.clone(),
        host: host.to_owned(),
        port: port.to_owned(),

        rewrite_index,
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
        include_index,
    };

    debug!("{:?}", &cfg);

    Ok(opts)
}

fn get_tag_info(args: &ProfileSettings) -> (ProfileName, String) {
    let release = args.release.is_some() && args.release.unwrap();

    let mut tag_target = ProfileName::Debug;
    if release {
        tag_target = ProfileName::Release;
    }

    if let Some(t) = &args.profile {
        if !t.is_empty() {
            tag_target = ProfileName::Custom(t.to_string());
        }
    }

    let target_dir = tag_target.to_string();
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

pub fn prepare(cfg: &Config, args: &ProfileSettings) -> Result<RuntimeOptions> {
    let (_, target_dir) = get_tag_info(args);

    // Handle profiles, eg: [profile.dist] that mutate the
    // arguments from config declarations
    let profiles = cfg.profile.as_ref().unwrap();
    if let Some(profile) = profiles.get(&target_dir) {
        let mut use_profile = profile.clone();

        if profile.profile.is_some() {
            return Err(Error::NoProfileInProfile);
        }

        if let Some(ref mut paths) = use_profile.paths.as_mut() {
            let build = cfg.build.as_ref().unwrap();
            let paths = prefix(&build.source, paths);
            debug!("profile paths {:?}", &paths);
            use_profile.paths = Some(paths);
        }

        let mut merged = super::merge::map::<ProfileSettings>(&use_profile, args)?;

        // Always update base to use the path separator. The declaration is
        // a URL path but internally we treat as a filesystem path.
        if let Some(ref base) = merged.base {
            merged.base = Some(utils::url::to_path_separator(base));
        }

        return with(cfg, &merged);
    }

    with(cfg, args)
}
