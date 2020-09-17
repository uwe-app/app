use std::fs;
use std::path::PathBuf;

use log::{debug, info};

use config::{Config, ProfileSettings};
use config::{ProfileName, RuntimeOptions, MENU};

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

    let mut base = args.target.clone();
    let target_dir = args.name.to_string();
    if !target_dir.is_empty() {
        let target_dir_buf = PathBuf::from(&target_dir);
        if target_dir_buf.is_absolute() {
            return Err(Error::ProfileNameAbsolute(target_dir));
        }
        base.push(target_dir);
        base = project.join(base);
    }

    let live = args.is_live();

    if live && release {
        return Err(Error::LiveReloadRelease);
    }

    let incremental = args.is_incremental();
    let pristine = args.is_pristine();

    if (pristine || args.is_force()) && base.exists() {
        info!("clean {}", base.display());
        fs::remove_dir_all(&base)?;
    }

    // Force is implied when live and incremental, the live
    // setting overrides the incremental behavior
    if live && incremental && !args.is_force() {
        args.force = Some(true);
    }

    require_output_dir(&base)?;

    if !source.exists() || !source.is_dir() {
        return Err(Error::NotDirectory(source.clone()));
    }

    // TODO: always use a layout?
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

    let mut settings = args.clone();
    if let Some(resources) = settings.resources.as_mut() {
        resources.prepare();
    }

    if let Some(ref book) = cfg.book {
        for (_k, item) in book.members.iter() {
            let book_path = source.join(&item.path);
            let book_menu = book_path.join(MENU);

            if !book_menu.exists() || !book_menu.is_file() {
                return Err(Error::NoBookMenu(book_menu, item.path.clone()));
            }
        } 
    }

    let opts = RuntimeOptions {
        project,
        source,
        output: settings.target.clone(),
        base,
        settings,
    };

    debug!("{:?}", &cfg);

    Ok(opts)
}

fn get_profile_source(cfg: &Config, args: &ProfileSettings) -> PathBuf {
    let base_dir = cfg.get_project();
    base_dir.join(&args.source.clone())
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

pub(crate) fn prepare(
    cfg: &Config,
    args: &ProfileSettings,
) -> Result<RuntimeOptions> {
    let name = to_profile(args);

    // Inherit the profile settings from the root
    let mut root = cfg.build.as_ref().unwrap().clone();
    let mut input = args.clone();

    // Handle profiles, eg: [profile.dist] that mutate the
    // arguments from config declarations
    let profiles = cfg.profile.as_ref().unwrap();
    if let Some(profile) = profiles.get(&name.to_string()) {
        let mut copy = profile.clone();
        root.append(&mut copy);

        if profile.profile.is_some() {
            return Err(Error::NoProfileInProfile);
        }

        from_cli(&mut root, &mut input);
        to_options(name, cfg, &mut root)
    } else {
        root.append(&mut input);
        from_cli(&mut root, &mut input);
        to_options(name, cfg, &mut root)
    }
}
