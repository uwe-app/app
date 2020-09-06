use std::collections::HashMap;
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use log::debug;

use crossbeam::channel;

use ignore::{WalkBuilder, WalkState};

use config::indexer::QueryList;
use config::link::{self, LinkOptions};
use config::{
    Config, FileInfo, FileOptions, LocaleMap, LocaleName, Page, RuntimeOptions,
};

use crate::locale::*;

use super::loader;
use super::{
    Collate, CollateInfo, Error, Resource, ResourceKind, ResourceOperation,
    Result,
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

pub async fn walk(
    req: CollateRequest<'_>,
    res: &mut CollateResult,
) -> Result<Vec<Error>> {
    let errors = find(&req, res).await?;
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
                    if let Some((lang, fallback)) =
                        get_locale_file_info(&path, &languages)
                    {
                        // Update the path for the new file
                        buf = fallback;
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
                        && FileInfo::is_page(&path, req.options);

                    if is_page {
                        if let Err(e) = add_page(req, info, &key, &path) {
                            let _ = tx.send(e);
                        }
                    } else {
                        // Store the primary layout
                        if let Some(ref layout) = req.options.settings.layout {
                            if key.starts_with(layout) {
                                info.layout = Some(Arc::clone(&key));
                                return WalkState::Continue;
                            }
                        }

                        if let Err(e) = add_other(req, info, &key) {
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
                let href = href(&src, req.options, false, None)?;
                link(&mut info, Arc::new(src), Arc::new(href))?;
            }
        }
    }
    Ok(())
}

pub fn series(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
) -> Result<()> {
    if let Some(ref series) = config.series {
        for (k, v) in series {
            let mut refs: Vec<Arc<PathBuf>> = Vec::new();
            v.pages
                .iter()
                .map(|p| {
                    if !p.starts_with(&options.source) {
                        return options.source.join(p);
                    }
                    p.to_path_buf()
                })
                .try_for_each(|p| {
                    if let Some(ref _page) = info.resolve(&p) {
                        let item = Arc::new(p.clone());
                        if refs.contains(&item) {
                            return Err(Error::DuplicateSeriesPage(
                                k.to_string(),
                                p.to_path_buf(),
                            ));
                        }
                        refs.push(item);
                        return Ok(());
                    }
                    Err(Error::NoSeriesPage(k.to_string(), p.to_path_buf()))
                })?;

            info.series.entry(k.to_string()).or_insert(refs);
        }
    }
    Ok(())
}

pub fn get_destination(
    file: &PathBuf,
    config: &Config,
    options: &RuntimeOptions,
) -> Result<PathBuf> {
    let mut info = FileInfo::new(&config, &options, file, false);

    let file_opts = FileOptions {
        exact: true,
        base_href: &options.settings.base_href,
        ..Default::default()
    };
    Ok(info.destination(&file_opts)?)
}

pub fn link(
    info: &mut CollateInfo,
    source: Arc<PathBuf>,
    href: Arc<String>,
) -> Result<()> {
    if let Some(existing) = info.links.reverse.get(&href) {
        return Err(Error::LinkCollision(
            href.to_string(),
            existing.to_path_buf(),
            source.to_path_buf(),
        ));
    }

    //println!("Link href {:?}", &href);
    info.links
        .reverse
        .entry(Arc::clone(&href))
        .or_insert(Arc::clone(&source));
    info.links.sources.entry(source).or_insert(href);
    Ok(())
}

pub fn href(
    file: &PathBuf,
    options: &RuntimeOptions,
    rewrite: bool,
    strip: Option<PathBuf>,
) -> Result<String> {
    let mut href_opts: LinkOptions = Default::default();
    href_opts.strip = strip;
    href_opts.rewrite = rewrite;
    href_opts.trailing = false;
    href_opts.include_index = true;
    link::absolute(file, options, href_opts).map_err(Error::from)
}

fn verify_query(list: &QueryList) -> Result<()> {
    let queries = list.to_vec();
    for q in queries {
        let each = q.each.is_some() && q.each.unwrap();
        if q.page.is_some() && each {
            return Err(Error::QueryConflict);
        }
    }
    Ok(())
}

fn add_page(
    req: &CollateRequest<'_>,
    mut info: &mut CollateInfo,
    key: &Arc<PathBuf>,
    path: &Path,
) -> Result<()> {
    let pth = path.to_path_buf();

    let mut page_info = loader::compute(path, req.config, req.options, true)?;

    if let Some(ref query) = page_info.query {
        verify_query(query)?;
        info.queries.push((query.clone(), Arc::clone(key)));
    }

    // Rewrite layouts relative to the source directory
    if let Some(ref layout) = page_info.layout {
        let layout_path = req.options.source.join(layout);
        if !layout_path.exists() {
            return Err(Error::NoLayout(layout_path, layout.clone()));
        }

        page_info.layout = Some(layout_path);
    }

    let mut file_info = FileInfo::new(req.config, req.options, &pth, false);

    let mut rewrite_index = req.options.settings.should_rewrite_index();
    // Override with rewrite-index page level setting
    if let Some(val) = page_info.rewrite_index {
        rewrite_index = val;
    }

    let file_opts = FileOptions {
        rewrite_index,
        base_href: &req.options.settings.base_href,
        ..Default::default()
    };

    let dest = file_info.destination(&file_opts)?;
    page_info.seal(
        &dest,
        req.config,
        req.options,
        &file_info,
        None,
        &info.lang,
    )?;

    if let Some(ref layout) = page_info.layout {
        // Register the layout
        info.layouts.insert(Arc::clone(key), layout.clone());
    }

    let href = href(&pth, req.options, rewrite_index, None)?;
    link(&mut info, Arc::clone(key), Arc::new(href.clone()))?;

    // Map permalinks to be converted to redirects later
    if let Some(ref permalink) = page_info.permalink {
        let key = permalink.trim_end_matches("/").to_string();

        if info.permalinks.contains_key(&key) {
            return Err(Error::DuplicatePermalink(key));
        }

        info.permalinks
            .insert(key, page_info.href.as_ref().unwrap().to_string());
    }

    // Collate feed pages
    if let Some(ref feed) = req.config.feed {
        for (name, cfg) in feed.channels.iter() {
            let href = page_info.href.as_ref().unwrap();
            if cfg.matcher.filter(href) {
                let items =
                    info.feeds.entry(name.to_string()).or_insert(vec![]);
                items.push(Arc::clone(key));
            }
        }
    }

    add_page_reference(info, req.config, req.options, key, dest, page_info);

    Ok(())
}

// Note that this is also used when creating synthetic pages from
// data sources, pagination etc.
pub fn add_page_reference(
    info: &mut CollateInfo,
    _config: &Config,
    _options: &RuntimeOptions,
    key: &Arc<PathBuf>,
    dest: PathBuf,
    page_info: Page,
) {
    let mut resource = Resource::new_page(dest);
    if let Some(ref render) = page_info.render {
        if !render {
            resource.set_operation(ResourceOperation::Copy);
        }
    }
    info.all.insert(Arc::clone(key), resource);
    info.resources.insert(Arc::clone(key));

    info.pages.entry(Arc::clone(key)).or_insert(page_info);
}

pub fn add_file(
    key: &Arc<PathBuf>,
    dest: PathBuf,
    href: String,
    info: &mut CollateInfo,
    _config: &Config,
    options: &RuntimeOptions,
) -> Result<()> {
    // Set up the default resource operation
    let mut op = if options.settings.is_release() {
        ResourceOperation::Copy
    } else {
        ResourceOperation::Link
    };

    // Allow the profile settings to control the resource operation
    if let Some(ref resources) = options.settings.resources {
        if resources.ignore.matcher.matches(&href) {
            op = ResourceOperation::Noop;
        } else if resources.symlink.matcher.matches(&href) {
            op = ResourceOperation::Link;
        } else if resources.copy.matcher.matches(&href) {
            op = ResourceOperation::Copy;
        }
    }

    let kind = get_file_kind(key, options);
    match kind {
        ResourceKind::File | ResourceKind::Asset => {
            info.resources.insert(Arc::clone(&key));
            link(info, Arc::clone(key), Arc::new(href))?;
        }
        _ => {}
    }

    info.all
        .insert(Arc::clone(key), Resource::new(dest, kind, op));

    Ok(())
}

fn get_file_kind(key: &Arc<PathBuf>, options: &RuntimeOptions) -> ResourceKind {
    let mut kind = ResourceKind::File;
    if key.starts_with(options.get_assets_path()) {
        kind = ResourceKind::Asset;
    } else if key.starts_with(options.get_partials_path()) {
        kind = ResourceKind::Partial;
    } else if key.starts_with(options.get_includes_path()) {
        kind = ResourceKind::Include;
    } else if key.starts_with(options.get_locales()) {
        kind = ResourceKind::Locale;
    } else if key.starts_with(options.get_data_sources_path()) {
        kind = ResourceKind::DataSource;
    }
    kind
}

fn add_other(
    req: &CollateRequest<'_>,
    info: &mut CollateInfo,
    key: &Arc<PathBuf>,
) -> Result<()> {
    let pth = key.to_path_buf();
    let dest = get_destination(&pth, req.config, req.options)?;
    let href = href(&pth, req.options, false, None)?;
    Ok(add_file(key, dest, href, info, req.config, req.options)?)
}
