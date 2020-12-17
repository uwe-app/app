use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use crossbeam::channel;
use ignore::{WalkBuilder, WalkState};
use log::debug;

use config::{
    href::UrlPath, plugin_cache::PluginCache, Config, RuntimeOptions,
};
use locale::{LocaleMap, LocaleName};

use crate::{
    builder::{to_href, PageBuilder},
    locale_utils::*,
    CollateInfo, Error, Resource, ResourceKind, ResourceOperation, Result,
};

pub struct CollateRequest<'a> {
    pub config: &'a Config,
    pub options: &'a RuntimeOptions,
    pub locales: &'a LocaleMap,
    pub plugins: Option<&'a PluginCache>,
}

pub struct CollateResult {
    pub inner: Arc<Mutex<CollateInfo>>,
    pub translations: Arc<Mutex<HashMap<LocaleName, CollateInfo>>>,
    pub errors: Vec<Error>,
}

impl CollateResult {
    pub fn new(lang: &str, base: &PathBuf, locales: &LocaleMap) -> Self {
        let translations = locales.alternate();

        let mut map: HashMap<LocaleName, CollateInfo> = HashMap::new();
        for lang in translations.iter() {
            let path = get_locale_target(lang, base, locales);
            let info = CollateInfo {
                lang: lang.to_string(),
                path,
                ..Default::default()
            };
            map.insert(lang.to_string(), info);
        }

        // Path for the fallback language
        let path = get_locale_target(lang, base, locales);

        Self {
            inner: Arc::new(Mutex::new(CollateInfo {
                lang: lang.to_string(),
                path,
                ..Default::default()
            })),
            translations: Arc::new(Mutex::new(map)),
            errors: Vec::new(),
        }
    }
}

impl TryInto<Vec<CollateInfo>> for CollateResult {
    type Error = Error;
    fn try_into(self) -> std::result::Result<Vec<CollateInfo>, Self::Error> {
        // Extract the primary fallback collation.
        let lock = Arc::try_unwrap(self.inner)
            .expect("Collate lock still has multiple owners");
        let info = lock.into_inner()?;

        let mut locales = vec![info];

        // Extract the translation collations.
        let lock = Arc::try_unwrap(self.translations)
            .expect("Collate translations lock still has multiple owners");
        let translations = lock.into_inner()?;
        for (_, v) in translations.into_iter() {
            locales.push(v);
        }

        Ok(locales)
    }
}

pub async fn walk(
    req: CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<Vec<Error>> {
    let errors = find(&req, res).await?;

    compute_links(&req, res)?;
    Ok(errors)
}

/// Determine the default layout name.
///
/// When a `main.hbs` file exists in the project layouts
/// directory the name will be `main` otherwise try to
/// use the `std::core::main` layout as the default.
pub fn layout_name(options: &RuntimeOptions) -> &str {
    let layouts_dir = options.source.join(config::LAYOUTS);
    let primary_layout = layouts_dir.join(config::LAYOUT_HBS);
    if primary_layout.exists() {
        config::MAIN
    } else {
        config::DEFAULT_LAYOUT_NAME
    }
}

async fn find(
    req: &CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<Vec<Error>> {
    let languages = req.locales.alternate();

    let engine = req.config.engine();
    let template_ext = engine.extension();

    let layouts_dir = req.options.source.join(config::LAYOUTS);
    let partials_dir = req.options.source.join(config::PARTIALS);

    let primary_layout = layouts_dir.join(config::LAYOUT_HBS);
    let layout_name = layout_name(&req.options);

    // Channel for collecting errors
    let (tx, rx) = channel::unbounded();

    WalkBuilder::new(&req.options.source)
        .follow_links(true)
        .build_parallel()
        .run(|| {
            Box::new(|result| {
                if let Ok(entry) = result {
                    let data = Arc::clone(&res.inner);
                    let translate_data = Arc::clone(&res.translations);
                    let mut info = &mut *data.lock().unwrap();
                    let mut translations = translate_data.lock().unwrap();

                    let path = entry.path();
                    let mut buf = path.to_path_buf();

                    // Check if this is a locale specific file by testing
                    // an extensions prefix,eg: `.fr.md` indicates this is
                    // a French language file.
                    if let Some((lang, normalized_path)) =
                        get_locale_file_info(&path, languages)
                    {
                        // Update the path for the new file
                        buf = normalized_path;
                        // Switch the collation to put the file into
                        info = translations.get_mut(&lang).unwrap();
                    }

                    debug!("Collate {} for {}", buf.display(), &info.lang);

                    let key = Arc::new(buf);

                    if path.extension() == Some(OsStr::new(template_ext)) {
                        // Partials are a convention that the parser will handle
                        if path.starts_with(&partials_dir) {
                            return WalkState::Continue;
                        }

                        // Configure the default layout to use a `layouts/main.hbs` file
                        if &*key == &primary_layout {
                            info.add_layout(
                                config::MAIN.to_string(),
                                Arc::clone(&key),
                            );
                        // Support alternative custom layouts by convention
                        } else if path.starts_with(&layouts_dir) {
                            let name = path
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string();
                            info.add_layout(name, Arc::clone(&key));
                        // Templates intermingled in the source tree
                        } else {
                            if let Err(e) =
                                add_template(info, req.config, req.options, key)
                            {
                                let _ = tx.send(Error::from(e));
                            }
                        }

                        return WalkState::Continue;
                    }

                    // Directories are stored in memory but do not represent pages
                    if path.is_dir() {
                        let res = Resource::new(
                            path.to_path_buf(),
                            ResourceKind::Dir,
                            ResourceOperation::Noop,
                        );
                        info.all.insert(Arc::clone(&key), res);
                        return WalkState::Continue;
                    }

                    let is_data_source =
                        key.starts_with(req.options.get_data_sources_path());

                    let is_page = !is_data_source
                        && path.is_file()
                        && req.options.is_page(&path);

                    if is_page {
                        if let Err(e) = add_page(
                            info,
                            req.config,
                            req.options,
                            req.plugins,
                            &key,
                            &path,
                            &layout_name,
                        ) {
                            let _ = tx.send(e);
                        }
                    } else {
                        if let Err(e) =
                            add_other(info, req.config, req.options, key)
                        {
                            let _ = tx.send(Error::from(e));
                        }
                    }
                }
                WalkState::Continue
            })
        });

    drop(tx);

    Ok(rx.iter().collect())
}

fn compute_links(
    req: &CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<()> {
    let mut info = res.inner.lock().unwrap();

    // Compute explicitly allowed links, typically this would be used
    // for synthetic files outside the system such as those generated
    // by hooks.
    if let Some(ref links) = req.config.link {
        if let Some(ref allow) = links.allow {
            for s in allow {
                let src = req.options.source.join(s.trim_start_matches("/"));
                let href = to_href(&src, req.options, false, None)?;
                info.link(Arc::new(src), Arc::new(href))?;
            }
        }
    }
    Ok(())
}

fn add_page(
    info: &mut CollateInfo,
    config: &Config,
    options: &RuntimeOptions,
    plugins: Option<&PluginCache>,
    key: &Arc<PathBuf>,
    path: &Path,
    layout_name: &str,
) -> Result<()> {
    let builder = PageBuilder::new(info, config, options, plugins, key, path)
        .compute()?
        .layout(layout_name)?
        .queries()?
        .seal()?
        .scripts()?
        .styles()?
        .layouts()?
        .link()?
        .permalinks()?
        .feeds()?;

    let (info, key, destination, page) = builder.build();

    //println!("Adding page with key {:?}", key);

    info.add_page(key, destination, Arc::new(RwLock::new(page)));

    Ok(())
}

fn add_other(
    info: &mut CollateInfo,
    _config: &Config,
    options: &RuntimeOptions,
    key: Arc<PathBuf>,
) -> Result<()> {
    let dest = options.destination().exact(true).build(&key)?;

    let href = to_href(&key, options, false, None)?;
    Ok(info.add_file(options, key, dest, href, None)?)
}

fn add_template(
    info: &mut CollateInfo,
    _config: &Config,
    options: &RuntimeOptions,
    key: Arc<PathBuf>,
) -> Result<()> {
    let path = key.canonicalize()?;
    info.add_template(Arc::new(path));
    Ok(())
}
