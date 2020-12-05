use std::fs;
use std::path::PathBuf;

use log::{debug, info};

use config::{
    script::ScriptAsset,
    tags::{link::LinkTag, script::ScriptTag},
    Config, ProfileName, ProfileSettings, RuntimeOptions,
};

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

    let project = cfg.project();
    let source = project.join(&args.source);

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

    let opts =
        RuntimeOptions::new(project.to_path_buf(), source, base, settings);

    debug!("{:#?}", &cfg);
    debug!("{:#?}", &opts);

    Ok(opts)
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
    if args.offline.is_some() {
        settings.offline = args.offline.clone();
    }
}

/// Prepare the live reload style and script.
fn prepare_live(cfg: &mut Config) -> Result<()> {
    let global_page = cfg.page.get_or_insert(Default::default());
    let style_tag = LinkTag::new_style_sheet(livereload::stylesheet(), None);
    global_page.links_mut().push(style_tag);

    let script_tag = ScriptTag::new(livereload::javascript());
    global_page.scripts_mut().push(ScriptAsset::Tag(script_tag));

    Ok(())
}

/// Prepare the site favicon.
fn prepare_icon(cfg: &mut Config, opts: &RuntimeOptions) -> Result<()> {
    let main_icon = cfg.icon_mut().take();
    let global_page = cfg.page.get_or_insert(Default::default());

    let icon_random = if !opts.settings.is_release() {
        Some(format!("?v={}", utils::generate_id(8)))
    } else {
        None
    };

    // Custom icon was defined
    if let Some(icon) = main_icon {
        let mut src = icon.to_string();
        let path = utils::url::to_path_separator(&src);
        let file = opts.source.join(&path);
        if !file.exists() || !file.is_file() {
            return Err(Error::NoMainIcon(src.to_string(), file));
        }
        if let Some(ref random) = icon_random {
            src.push_str(random);
        }
        let icon_tag = LinkTag::new_icon(src);
        global_page.links_mut().push(icon_tag);
    } else {
        let mut src = Config::default_icon_url().to_string();
        let path = utils::url::to_path_separator(&src);
        let file = opts.source.join(&path);
        if file.exists() && file.is_file() {
            if let Some(ref random) = icon_random {
                src.push_str(random);
            }
            let icon_tag = LinkTag::new_icon(src);
            global_page.links_mut().push(icon_tag);
        } else {
            global_page.links_mut().push(Config::default_icon());
        }
    }

    Ok(())
}

/// Prepare the main style sheet.
fn prepare_style(cfg: &mut Config, opts: &RuntimeOptions) -> Result<()> {
    let main_style = cfg.style_mut().take();
    let global_page = cfg.page.get_or_insert(Default::default());

    // Custom style was defined
    if let Some(style) = main_style {
        // Check main style exists
        if let Some(src) = style.source() {
            let path = utils::url::to_path_separator(src);
            let file = opts.source.join(&path);
            if !file.exists() || !file.is_file() {
                return Err(Error::NoMainStyle(src.to_string(), file));
            }
        }

        global_page.links_mut().push(style.to_tag().to_link_tag());

    // Using the style convention
    } else {
        let asset = Config::default_style();
        let main_style_path = utils::url::to_path_separator(asset.source());
        let main_style_file = opts.source.join(&main_style_path);
        // Add a primary style sheet by convention if it exists
        if main_style_file.exists() && main_style_file.is_file() {
            let href = utils::url::to_href_separator(
                main_style_file.strip_prefix(&opts.source)?,
            );
            // NOTE: must start with a slash for URLs on 404 error page
            let href = format!("/{}", href);
            let style_tag = LinkTag::new_style_sheet(href, None);
            global_page.links_mut().push(style_tag);
        }
    }

    Ok(())
}

/// Prepare the main script.
fn prepare_script(cfg: &mut Config, opts: &RuntimeOptions) -> Result<()> {
    let main_script = cfg.script_mut().take();
    let global_page = cfg.page.get_or_insert(Default::default());

    // Custom script was defined
    if let Some(script) = main_script {
        // Check main script exists
        if let Some(src) = script.source() {
            let path = utils::url::to_path_separator(src);
            let file = opts.source.join(&path);
            if !file.exists() || !file.is_file() {
                return Err(Error::NoMainScript(src.to_string(), file));
            }
        }

        global_page.scripts_mut().push(script);
    // Using the script convention
    } else {
        let asset = Config::default_script();
        let main_script_path = utils::url::to_path_separator(
            asset.to_tag().source().as_ref().unwrap(),
        );
        let main_script_file = opts.source.join(&main_script_path);
        if main_script_file.exists() && main_script_file.is_file() {
            let href = utils::url::to_href_separator(
                main_script_file.strip_prefix(&opts.source)?,
            );
            // NOTE: must start with a slash for URLs on 404 error page
            let href = format!("/{}", href);
            let script_tag = ScriptTag::new(href);
            global_page.scripts_mut().push(ScriptAsset::Tag(script_tag));
        }
    }

    Ok(())
}

pub(crate) async fn prepare(
    cfg: &mut Config,
    args: &ProfileSettings,
) -> Result<RuntimeOptions> {
    let name = to_profile(args);

    // Inherit the profile settings from the root
    let mut root = cfg.build.as_ref().unwrap().clone();
    let mut input = args.clone();

    // Handle profiles, eg: [profile.dist] that mutate the
    // arguments from config declarations
    let profiles = cfg.profile.as_ref().unwrap();
    let opts = if let Some(profile) = profiles.get(&name.to_string()) {
        let mut copy = profile.clone();
        root.append(&mut copy);

        if profile.profile.is_some() {
            return Err(Error::NoProfileInProfile);
        }

        from_cli(&mut root, &mut input);
        to_options(name, cfg, &mut root)?
    } else {
        root.append(&mut input);
        from_cli(&mut root, &mut input);
        to_options(name, cfg, &mut root)?
    };

    // Configure project level hooks
    let project = cfg.project().clone();
    if let Some(hooks) = cfg.hooks.as_mut() {
        hooks.prepare(&opts.source, &project)?;
    }

    let website = opts.settings.get_canonical_url(cfg)?;
    cfg.set_website(website);

    if opts.settings.is_live() {
        prepare_live(cfg)?;
    }

    prepare_icon(cfg, &opts)?;
    prepare_style(cfg, &opts)?;
    prepare_script(cfg, &opts)?;

    Ok(opts)
}
