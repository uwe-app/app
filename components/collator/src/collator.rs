use std::collections::HashMap;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use crossbeam::channel;
use ignore::{WalkBuilder, WalkState};
use log::debug;

use config::{Config, LayoutReference, MenuEntry, RuntimeOptions};
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
}

pub struct CollateResult {
    pub inner: Arc<Mutex<CollateInfo>>,
    pub translations: Arc<Mutex<HashMap<LocaleName, CollateInfo>>>,
    pub errors: Vec<Error>,
}

impl CollateResult {
    pub fn new(lang: &str, base: &PathBuf, locales: &LocaleMap) -> Self {
        let translations = locales.get_translations();

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

fn add_menu(
    info: &mut CollateInfo,
    _config: &Config,
    options: &RuntimeOptions,
    key: &Arc<PathBuf>,
) -> Result<()> {
    let url_path =
        utils::url::to_href_separator(key.strip_prefix(&options.source)?);

    // NOTE: use the parent directory as the menu key
    // NOTE: if possible
    let name = if let Some(parent) = key.parent() {
        parent.to_string_lossy().into_owned()
    } else {
        key.to_string_lossy().into_owned()
    };

    // Inject the menu entry for processing later.
    let entry = MenuEntry::new(name, url_path);
    info.graph.menus.sources.insert(Arc::new(entry), Vec::new());

    Ok(())
}

pub async fn walk(
    req: CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<Vec<Error>> {
    let errors = find(&req, res).await?;

    compute_layouts(&req, res)?;
    compute_links(&req, res)?;
    Ok(errors)
}

async fn find(
    req: &CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<Vec<Error>> {
    let languages = req.locales.get_translations();

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

                    // Detect special handling for MENU.md files.
                    let is_menu = is_page && MenuEntry::is_menu(&key);

                    if is_menu {
                        if let Err(e) =
                            add_menu(info, req.config, req.options, &key)
                        {
                            let _ = tx.send(e);
                        }
                    } else if is_page {
                        if let Err(e) =
                            add_page(info, req.config, req.options, &key, &path)
                        {
                            let _ = tx.send(e);
                        }
                    } else {
                        // Store the primary layout
                        if let Some(ref layout) = req.options.settings.layout {
                            match layout {
                                LayoutReference::File(ref file) => {
                                    if key.starts_with(file) {

                                        // Configure the default layout from a `layout.hbs` file
                                        info.add_layout(
                                            config::DEFAULT_LAYOUT_NAME.to_string(),
                                            Arc::clone(&key));

                                        return WalkState::Continue;
                                    }
                                }
                                _ => {}
                            }
                        }

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

fn compute_layouts(
    req: &CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<()> {
    let mut info = res.inner.lock().unwrap();

    // Compute layout paths relative to the source directory.
    if let Some(ref layouts) = req.config.layout {
        for (k, v) in layouts.iter() {
            let path = req
                .options
                .source
                .join(v.strip_prefix(&req.options.source)?);
            info.layouts.insert(k.clone(), Arc::new(path));
        }
    }

    Ok(())
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
    key: &Arc<PathBuf>,
    path: &Path,
) -> Result<()> {
    let builder = PageBuilder::new(info, config, options, key, path)
        .compute()?
        .queries()?
        .layouts()?
        .seal()?
        .link()?
        .permalinks()?
        .feeds()?;

    let (info, key, destination, mut page) = builder.build();

    if let Some(menu) = page.menu.as_mut() {
        // Verify file references as early as possible
        for (k, v) in menu.entries.iter_mut() {
            v.verify_files(&options.source)?;

            let mut def = v.clone();
            // Assign the key name so we can use it
            // later when re-assigning the compiled value
            def.name = k.clone();

            let entries = info
                .graph
                .menus
                .sources
                .entry(Arc::new(def))
                .or_insert(vec![]);
            entries.push(Arc::clone(key));
        }
    }

    if let Some(ref book) = config.book {
        for (k, item) in book.members.iter() {
            let p = options.source.join(&item.path);
            if key.starts_with(p) && !MenuEntry::is_menu(&key) {
                // All pages inherit the draft status from the book.
                if item.draft.is_some() {
                    page.draft = item.draft.clone();
                }
                let files =
                    info.books.entry(k.to_string()).or_insert(Vec::new());
                files.push(Arc::clone(key));
            }
        }
    }

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
