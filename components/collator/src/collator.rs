use std::path::{Path, PathBuf};
use std::convert::TryInto;
use std::sync::{Arc, Mutex};

use ignore::{WalkBuilder, WalkState};

use config::{Config, RuntimeOptions, FileInfo, FileOptions};
use config::link::{self, LinkOptions};

use super::{Error, Result, CollateInfo};

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
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CollateInfo { ..Default::default() } ))
        }    
    }
}

impl TryInto<CollateInfo> for CollateResult {
    type Error = Error;
    fn try_into(self) -> std::result::Result<CollateInfo, Self::Error> {
        let lock = Arc::try_unwrap(self.inner)
            .expect("Collate lock still has multiple owners");
        let info = lock.into_inner()?;
        Ok(info)
    }
}

pub async fn walk(req: CollateRequest<'_>, res: &mut CollateResult) -> Result<()> {
    find(&req, res).await?;
    compute_links(&req, res)?;
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

fn get_destination(file: &PathBuf, config: &Config, options: &RuntimeOptions) -> Result<PathBuf> {
    let mut info = FileInfo::new(
        &config,
        &options,
        file,
        false,
    );

    let file_opts = FileOptions {
        exact: true,
        base_href: &options.settings.base_href,
        ..Default::default()
    };
    Ok(info.destination(&file_opts)?)
}

pub fn link(info: &mut CollateInfo, source: Arc<PathBuf>, href: Arc<String>) -> Result<()> {
    //println!("Link href {:?}", &href);
    info.links.reverse.entry(Arc::clone(&href)).or_insert(Arc::clone(&source));
    info.links.sources.entry(source).or_insert(href);
    Ok(())
}

pub fn href(file: &PathBuf, options: &RuntimeOptions, rewrite: bool, strip: Option<PathBuf>) -> Result<String> {
    let mut href_opts: LinkOptions = Default::default();
    href_opts.strip = strip;
    href_opts.rewrite = rewrite;
    href_opts.trailing = false;
    href_opts.include_index = true;
    link::absolute(file, options, href_opts).map_err(Error::from)
}

fn add_page(
    req: &CollateRequest<'_>,
    mut info: &mut CollateInfo,
    key: &Arc<PathBuf>,
    path: &Path) -> Result<()> {

    let pth = path.to_path_buf();

    let mut page_info = loader::compute(&path, req.config, req.options, true)?;

    if let Some(ref query) = page_info.query {
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

    let mut file_info = FileInfo::new(
        req.config,
        req.options,
        &pth,
        false,
    );

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
    page_info.seal(&dest, req.config, req.options, &file_info)?;

    if let Some(ref layout) = page_info.layout {
        // Register the layout
        info.layouts.insert(Arc::clone(key), layout.clone());
    }

    let href = href(&pth, req.options, rewrite_index, None)?;
    link(&mut info, Arc::clone(key), Arc::new(href.clone()))?;

    info.pages.entry(Arc::clone(key)).or_insert(page_info);

    Ok(())
}


fn add_other(
    req: &CollateRequest<'_>,
    mut info: &mut CollateInfo,
    key: &Arc<PathBuf>,
    path: &Path,
    is_page: bool) -> Result<()> {

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
        let href = href(&pth, req.options, false, Some(req.options.get_resources_path()))?;
        link(&mut info, Arc::clone(key), Arc::new(href))?;
        info.resources.push(Arc::clone(key));
    } else if key.starts_with(req.options.get_locales()) {
        info.locales.push(Arc::clone(key));
    } else if key.starts_with(req.options.get_data_sources_path()) {
        info.data_sources.push(Arc::clone(key));
    } else if !is_page {
        let href = href(&pth, req.options, false, None)?;
        link(&mut info, Arc::clone(key), Arc::new(href))?;
        let dest = get_destination(&pth, req.config, req.options)?;
        info.other.entry(Arc::clone(key)).or_insert(dest);
    }

    Ok(())
}

async fn find(req: &CollateRequest<'_>, res: &mut CollateResult) -> Result<()> {
    let walk_filters = if req.filter {
        config::filter::get_filters(req.options, req.config)
    } else { Vec::new() };

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
                    let buf = path.to_path_buf();

                    let data = Arc::clone(&res.inner);
                    let mut info = data.lock().unwrap();

                    let pth = buf.clone();
                    let key = Arc::new(buf);

                    let is_data_source = key.starts_with(req.options.get_data_sources_path());
                    let is_page = !is_data_source && pth.is_file() && FileInfo::is_page(&path, req.options);

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
            }
        )
    });
    Ok(())
}

