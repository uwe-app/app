use std::convert::TryInto;
use std::sync::{Arc, Mutex};

use ignore::{WalkBuilder, WalkState};

use config::{Page, Config, RuntimeOptions, FileInfo, FileOptions};

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
    find(req, res).await?;
    Ok(())
}

async fn find(req: CollateRequest<'_>, res: &mut CollateResult) -> Result<()> {
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
        .build_parallel()
        .run(|| {
            Box::new(|result| {
                if let Ok(entry) = result {
                    let path = entry.path();
                    let buf = path.to_path_buf();
                    let mut page: Option<Page> = None;

                    let data = Arc::clone(&res.inner);
                    let mut info = data.lock().unwrap();

                    if buf.is_file() && FileInfo::is_page(&path, req.options) {
                        let result = loader::compute(&path, req.config, req.options, true);
                        match result {
                            Ok(mut page_info) => {

                                // Rewrite layouts relative to the source directory
                                if let Some(ref layout) = page_info.layout {
                                    let layout_path = req.options.source.join(layout);
                                    if !layout_path.exists() {
                                        info.errors.push(Error::NoLayout(layout_path, layout.clone()));
                                        return WalkState::Continue;
                                    }

                                    page_info.layout = Some(layout_path);
                                }

                                let mut file_info = FileInfo::new(
                                    req.config,
                                    req.options,
                                    &buf,
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

                                if let Err(e) = file_info.destination(&file_opts) {
                                    info.errors.push(Error::from(e));
                                    return WalkState::Continue;
                                }

                                if let Err(e) = page_info.seal(req.config, req.options, &file_info) {
                                    info.errors.push(Error::from(e));
                                    return WalkState::Continue;
                                }

                                page = Some(page_info);
                            }
                            Err(e) => {
                                info.errors.push(Error::from(e));
                                return WalkState::Continue;
                            }
                        }
                    }

                    let key = Arc::new(buf);

                    if key.is_dir() {
                        info.dirs.push(Arc::clone(&key));
                    } else if page.is_some() {

                        let page_info = page.as_ref().unwrap();
                        if let Some(ref layout) = page_info.layout {
                            // Register the layout
                            info.layouts.insert(Arc::clone(&key), layout.clone());
                        }

                        info.pages.push(Arc::clone(&key));
                    } else {
                        if !req.filter {

                            // TODO: store the layout?
                            if let Some(ref layout) = req.options.settings.layout {
                                if key.starts_with(layout) {
                                    info.layout = Some(Arc::clone(&key));
                                    return WalkState::Continue;
                                }
                            }

                            if key.starts_with(req.options.get_assets_path()) {
                                info.assets.push(Arc::clone(&key));
                            } else if key.starts_with(req.options.get_partials_path()) {
                                info.partials.push(Arc::clone(&key));
                            } else if key.starts_with(req.options.get_includes_path()) {
                                info.includes.push(Arc::clone(&key));
                            } else if key.starts_with(req.options.get_resources_path()) {
                                info.resources.push(Arc::clone(&key));
                            } else if key.starts_with(req.options.get_locales()) {
                                info.locales.push(Arc::clone(&key));
                            } else if key.starts_with(req.options.get_data_sources_path()) {
                                info.data_sources.push(Arc::clone(&key));
                            } else if key.starts_with(req.options.get_short_codes_path()) {
                                info.short_codes.push(Arc::clone(&key));
                            } else {
                                info.other.push(Arc::clone(&key));
                            }
                        }

                        info.files.push(Arc::clone(&key));
                    }

                    info.all.entry(key).or_insert(page);
                }
                WalkState::Continue
            }
        )
    });
    Ok(())
}

