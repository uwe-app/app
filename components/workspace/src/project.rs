use std::fs;
use std::path::PathBuf;

use log::{debug, info};

use config::{Config, ProfileSettings};
use config::{ProfileName, RuntimeOptions};

use crate::{Error, Result};

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

fn to_options(
    name: ProfileName,
    cfg: &Config,
    args: &mut ProfileSettings,
) -> Result<RuntimeOptions> {
    args.name = name.clone();
    args.set_defaults();

    // Always update base to use the path separator. The declaration is
    // a URL path but internally we treat as a filesystem path.
    if let Some(ref base) = args.base {
        args.base = Some(utils::url::to_path_separator(base));
    }

    let project = cfg.get_project();

    let source = get_profile_source(cfg, args);
    let release = args.is_release();

    let mut target = args.target.clone();
    let target_dir = args.name.to_string();
    if !target_dir.is_empty() {
        let target_dir_buf = PathBuf::from(&target_dir);
        if target_dir_buf.is_absolute() {
            return Err(Error::ProfileNameAbsolute(target_dir));
        }
        target.push(target_dir);
        target = project.join(target);
    }

    let live = args.is_live();

    if live && release {
        return Err(Error::LiveReloadRelease);
    }

    let incremental = args.is_incremental();
    let pristine = args.is_pristine();

    if (pristine || args.is_force()) && target.exists() {
        info!("clean {}", target.display());
        fs::remove_dir_all(&target)?;
    }

    // Force is implied when live and incremental, the live
    // setting overrides the incremental behavior
    if live && incremental && !args.is_force() {
        args.force = Some(true);
    }

    require_output_dir(&target)?;

    let serve = cfg.serve.as_ref().unwrap();
    if args.host.is_none() {
        args.host = Some(serve.host.clone());
    }

    if args.port.is_none() {
        args.port = Some(serve.port.clone());
    }

    if !source.exists() || !source.is_dir() {
        return Err(Error::NotDirectory(source.clone()));
    }

    if args.should_use_layout() {
        if let Some(ref layout) = args.layout {
            let location = source.clone().join(layout);
            if !location.exists() || !location.is_file() {
                return Err(Error::NoLayout(location.to_path_buf()));
            }
            args.layout = Some(location);
        }
    }

    if let Some(ref mut paths) = args.paths.as_mut() {
        let paths = prefix(&source, paths);
        for p in paths.iter() {
            if !p.exists() {
                return Err(Error::NoFilter(p.to_path_buf()));
            }
        }
        debug!("Profile paths {:?}", &paths);
        args.paths = Some(paths);
    }

    // Append render extension shortcuts to the list of types
    if let Some(type_defs) = args.types.as_mut() {
        if let Some(ref render) = args.extend {
            for ext in render {
                type_defs.types.insert(ext.to_string(), Default::default());
            }
        }
    }

    let opts = RuntimeOptions {
        lang: cfg.lang.clone(),
        project,
        source,
        output: args.target.clone(),
        base: target.clone(),
        settings: args.clone(),
        target,
        locales: Default::default(),
    };

    //println!("Got settings {:#?}", args);
    debug!("{:?}", &cfg);

    Ok(opts)
}

fn get_profile_source(cfg: &Config, args: &ProfileSettings) -> PathBuf {
    let base_dir = cfg.get_project();
    base_dir.join(&args.source)
}

fn to_profile(args: &ProfileSettings) -> ProfileName {
    let release = args.is_release();

    let mut target_profile = ProfileName::Debug;
    if release {
        target_profile = ProfileName::Release;
    }

    if let Some(t) = &args.profile {
        if !t.is_empty() {
            target_profile = ProfileName::from(t.to_string());
        }
    }

    //let target_dir = target_profile.to_string();
    target_profile
}

// Map a set of paths making them relative to the source, used when
// paths are defined in the `paths` definition of a profile in the configuration.
//
// When we get paths from the command line there is no need to prefix them.
fn prefix(source: &PathBuf, paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|p| {
            if !p.starts_with(source) {
                return source.join(p);
            }
            p.to_path_buf()
        })
        .collect::<Vec<_>>()
}

fn from_cli(settings: &mut ProfileSettings, args: &mut ProfileSettings) {
    // WARN: We cannot merge from args here otherwise we clobber
    // WARN: other settings from the arg defaults so we
    // WARN: manually override from command line arguments.
    if args.live.is_some() {
        settings.live = args.live;
    }
    if args.release.is_some() {
        settings.release = args.release;
    }
    if args.host.is_some() {
        settings.host = args.host.clone();
    }
    if args.port.is_some() {
        settings.port = args.port;
    }
}

pub fn prepare(cfg: &Config, args: &mut ProfileSettings) -> Result<RuntimeOptions> {
    let name = to_profile(args);

    // Inherit the profile settings from the root
    let root = cfg.build.as_ref().unwrap().clone();

    // Handle profiles, eg: [profile.dist] that mutate the
    // arguments from config declarations
    let profiles = cfg.profile.as_ref().unwrap();
    if let Some(ref profile) = profiles.get(&name.to_string()) {
        let mut copy = profile.clone();
        let mut merged = super::merge::map::<ProfileSettings>(&root, &mut copy)?;

        if profile.profile.is_some() {
            return Err(Error::NoProfileInProfile);
        }

        from_cli(&mut merged, args);
        to_options(name, cfg, &mut merged)
    } else {
        let mut merged = super::merge::map::<ProfileSettings>(&root, args)?;
        from_cli(&mut merged, args);
        to_options(name, cfg, &mut merged)
    }
}
