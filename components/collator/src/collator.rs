use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use ignore::{WalkBuilder, WalkState};

use config::indexer::QueryList;
use config::link::{self, LinkOptions};
use config::{Config, FileInfo, FileOptions, RuntimeOptions, LocaleMap, Page};

use super::loader;
use super::manifest::Manifest;
use super::{CollateInfo, Error, Result};

pub struct CollateRequest<'a> {
    // When filter is active then only `all`, `pages`, `files` and `dirs`
    // will be filled otherwise we organize by all the recognized types.
    pub filter: bool,
    pub config: &'a Config,
    pub options: &'a RuntimeOptions,
}

#[derive(Debug)]
pub struct CollateResult {
    pub inner: Arc<Mutex<CollateInfo>>,
}

impl CollateResult {
    pub fn new(manifest: Option<Manifest>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CollateInfo {
                manifest,
                ..Default::default()
            })),
        }
    }
}

impl TryInto<CollateInfo> for CollateResult {
    type Error = Error;
    fn try_into(self) -> std::result::Result<CollateInfo, Self::Error> {
        let lock = Arc::try_unwrap(self.inner).expect("Collate lock still has multiple owners");
        let info = lock.into_inner()?;
        Ok(info)
    }
}

fn is_locale_stem(names: &Vec<&str>, stem: &str) -> bool {
    for name in names {
        let ext = format!(".{}", name);
        if stem.ends_with(&ext) {
            return true 
        }
    }
    false
}

struct LocalePage {
    locale_id: String,
    page: Page,
    fallback: PathBuf,
    path: PathBuf,
}

// Localize logic involves another pass as we can't guarantee the order
// that pages are discovered so this allows us to ensure we have page 
// data for the default fallback locale before we assign page data for 
// locales specific pages. Which allows us to inherit page data from the 
// fallback page.
pub async fn localize(
    config: &Config,
    options: &RuntimeOptions,
    locales: &LocaleMap,
    info: &mut CollateInfo) -> Result<()> {

    let locale_names = locales.map.keys()
        .map(|k| k.as_str())
        .collect::<Vec<_>>();

    // As we remove locale-specific pages we cache them to be injected
    // into the locale_pages map
    let mut cache: Vec<LocalePage> = Vec::new();

    for (path, page) in info.pages.iter() {
        if let Some(ext) = path.extension() {
            let ext = ext.to_str().unwrap();
            if let Some(stem) = path.file_stem() {
                let stem = stem.to_str().unwrap();
                if is_locale_stem(&locale_names, stem) {
                    let stem_path = Path::new(stem);
                    let locale_id = stem_path.extension().unwrap().to_str().unwrap();
                    let parent_stem = stem_path.file_stem().unwrap().to_str().unwrap();
                    let fallback_name = format!("{}.{}", parent_stem, ext);
                    let fallback = path.parent().unwrap().join(fallback_name);
                    let tmp = LocalePage {
                        locale_id: locale_id.to_string(),
                        page: page.clone(),
                        fallback,
                        path: path.to_path_buf(),
                    };
                    cache.push(tmp);
                }
            } 
        } 
    }

    for entry in cache {
        let lang = entry.locale_id.clone();
        let map = info.locale_pages.entry(entry.locale_id).or_insert(HashMap::new());
        let mut page_info = entry.page;
        let use_fallback = page_info.fallback.is_some() && page_info.fallback.unwrap();

        // Inherit from the fallback page when it exists
        if let Some(fallback_page) = info.pages.get(&entry.fallback) {
            let file_context = fallback_page.file.as_ref().unwrap();
            let source = file_context.source.clone();
            // NOTE: Must clone the fallback page
            let mut fallback_page = fallback_page.clone();

            let template = if use_fallback {
                fallback_page.file.as_ref().unwrap().template.clone()
            }else {
                page_info.file.as_ref().unwrap().template.clone()
            };

            let mut tmp: Page = Default::default();

            tmp.append(&mut fallback_page);
            tmp.append(&mut page_info);

            let mut rewrite_index = options.settings.should_rewrite_index();
            // Override with rewrite-index page level setting
            if let Some(val) = tmp.rewrite_index {
                rewrite_index = val;
            }

            // Must seal() again so the file paths are correct
            let mut file_info = FileInfo::new(config, options, &source, false);
            let file_opts = FileOptions {
                rewrite_index,
                base_href: &options.settings.base_href,
                ..Default::default()
            };
            let dest = file_info.destination(&file_opts)?;
            tmp.seal(&dest, config, options, &file_info, Some(template))?;
            tmp.set_language(&lang);

            // Ensure we are putting the file in the correct locale specific location
            let locale_target = options.target.join(&lang);
            tmp.rewrite_target(&options.target, &locale_target)?;

            page_info = tmp;

        }
        map.insert(Arc::new(entry.fallback), page_info);

        info.remove_page(&entry.path);
    }

    //for (k, _v) in info.pages.iter() {
        //println!("Got page key {:?}", k);
    //}

    //println!("Locale pages {:#?}", info.locale_pages);

    Ok(())
}


pub async fn walk(req: CollateRequest<'_>, res: &mut CollateResult) -> Result<()> {
    find(&req, res).await?;
    compute_links(&req, res)?;
    Ok(())
}

pub fn series(config: &Config, options: &RuntimeOptions, info: &mut CollateInfo) -> Result<()> {
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
                    if let Some(ref _page) = info.pages.get(&p) {
                        let item = Arc::new(p.clone());
                        if refs.contains(&item) {
                            return Err(Error::DuplicateSeriesPage(k.to_string(), p.to_path_buf()));
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

fn compute_links(req: &CollateRequest<'_>, res: &mut CollateResult) -> Result<()> {
    let data = Arc::clone(&res.inner);
    let mut info = data.lock().unwrap();

    // Compute explicitly allowed links, typically this would be used
    // for synthetic files outside the system such as those generated
    // by hooks.
    if let Some(ref links) = req.config.link {
        if let Some(ref allow) = links.allow {
            for s in allow {
                let s = s.trim_start_matches("/");
                let src = req.options.source.join(s);
                let href = href(&src, req.options, false, None)?;
                link(&mut info, Arc::new(src), Arc::new(href))?;
            }
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

pub fn link(info: &mut CollateInfo, source: Arc<PathBuf>, href: Arc<String>) -> Result<()> {
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

    let mut page_info = loader::compute(&path, req.config, req.options, true)?;

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
    page_info.seal(&dest, req.config, req.options, &file_info, None)?;

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
            return Err(Error::DuplicatePermalink(key)) 
        }

        info.permalinks.insert(
            key,
            page_info.href.as_ref().unwrap().to_string());
    }

    // Collate feed pages
    if let Some(ref feed) = req.config.feed {
        for (name, cfg) in feed.channels.iter() {
            let href = page_info.href.as_ref().unwrap();
            if cfg.filter(href) {
                let items = info.feeds
                    .entry(name.to_string())
                    .or_insert(vec![]); 
                items.push(Arc::clone(key));
            }
        }
    }

    info.pages.entry(Arc::clone(key)).or_insert(page_info);
    info.targets.entry(Arc::clone(key)).or_insert(dest);

    Ok(())
}

pub fn add_file(
    key: &Arc<PathBuf>,
    dest: PathBuf,
    href: String,
    mut info: &mut CollateInfo,
    _config: &Config,
    _options: &RuntimeOptions,
) -> Result<()> {
    //let pth = key.to_path_buf();
    link(&mut info, Arc::clone(key), Arc::new(href))?;

    info.other.entry(Arc::clone(key)).or_insert(dest.clone());
    info.targets.entry(Arc::clone(key)).or_insert(dest);

    Ok(())
}

fn add_other(
    req: &CollateRequest<'_>,
    mut info: &mut CollateInfo,
    key: &Arc<PathBuf>,
    path: &Path,
    is_page: bool,
) -> Result<()> {
    let pth = path.to_path_buf();

    // This falls through so it is captured as part
    // of the other group too but we track these files
    // for reporting purposes
    if key.starts_with(req.options.get_assets_path()) {
        info.assets.push(Arc::clone(key));
    }

    if key.starts_with(req.options.get_partials_path()) {
        info.partials.push(Arc::clone(key));
    } else if key.starts_with(req.options.get_includes_path()) {
        info.includes.push(Arc::clone(key));
    } else if key.starts_with(req.options.get_resources_path()) {
        let href = href(
            &pth,
            req.options,
            false,
            Some(req.options.get_resources_path()),
        )?;
        link(&mut info, Arc::clone(key), Arc::new(href))?;
        info.resources.push(Arc::clone(key));
    } else if key.starts_with(req.options.get_locales()) {
        info.locales.push(Arc::clone(key));
    } else if key.starts_with(req.options.get_data_sources_path()) {
        info.data_sources.push(Arc::clone(key));
    } else if !is_page {
        let dest = get_destination(&pth, req.config, req.options)?;
        let href = href(&pth, req.options, false, None)?;
        add_file(key, dest, href, info, req.config, req.options)?;

        /*

        let href = href(&pth, req.options, false, None)?;
        link(&mut info, Arc::clone(key), Arc::new(href))?;

        let dest = get_destination(&pth, req.config, req.options)?;
        info.other.entry(Arc::clone(key)).or_insert(dest.clone());
        info.targets.entry(Arc::clone(key)).or_insert(dest);
        */
    }

    Ok(())
}

async fn find(req: &CollateRequest<'_>, res: &mut CollateResult) -> Result<()> {
    let walk_filters = if req.filter {
        config::filter::get_filters(req.options, req.config)
    } else {
        Vec::new()
    };

    WalkBuilder::new(&req.options.source)
        .filter_entry(move |e| {
            let path = e.path();
            if walk_filters.contains(&path.to_path_buf()) {
                return false;
            }
            true
        })
        .follow_links(true)
        .build_parallel()
        .run(|| {
            Box::new(|result| {
                if let Ok(entry) = result {
                    let path = entry.path();

                    let data = Arc::clone(&res.inner);
                    let mut info = data.lock().unwrap();

                    let key = Arc::new(path.to_path_buf());

                    let is_data_source = key.starts_with(req.options.get_data_sources_path());
                    let is_page =
                        !is_data_source && path.is_file() && FileInfo::is_page(&path, req.options);

                    if is_page {
                        if let Err(e) = add_page(req, &mut *info, &key, &path) {
                            info.errors.push(e);
                            return WalkState::Continue;
                        }
                    }

                    if key.is_dir() {
                        info.dirs.push(Arc::clone(&key));
                    } else {
                        if !req.filter {
                            // Store the primary layout
                            if let Some(ref layout) = req.options.settings.layout {
                                if key.starts_with(layout) {
                                    info.layout = Some(Arc::clone(&key));
                                    return WalkState::Continue;
                                }
                            }

                            if let Err(e) = add_other(req, &mut *info, &key, &path, is_page) {
                                info.errors.push(Error::from(e));
                                return WalkState::Continue;
                            }
                        }

                        info.files.push(Arc::clone(&key));
                    }

                    info.all.push(key);
                }
                WalkState::Continue
            })
        });
    Ok(())
}
