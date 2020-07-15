use std::convert::TryInto;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::collections::HashMap;

use thiserror::Error;
use ignore::{WalkBuilder, WalkState};

use config::{Page, Config, FileInfo, RuntimeOptions};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Poison(#[from] std::sync::PoisonError<CollateInfo>),

    #[error(transparent)]
    Loader(#[from] loader::Error),
}

type Result<T> = std::result::Result<T, Error>;

pub struct CollateRequest<'a> {
    // When filter is active then only `all`, `pages`, `files` and `dirs`
    // will be filled otherwise we organize by all the recognized types.
    pub filter: bool,
    pub config: &'a Config,
    pub options: &'a RuntimeOptions,
}

#[derive(Debug, Default)]
pub struct CollateInfo {
    pub errors: Vec<Error>,
    pub all: HashMap<Arc<PathBuf>, Option<Page>>,
    pub pages: Vec<Arc<PathBuf>>,
    pub dirs: Vec<Arc<PathBuf>>,
    pub files: Vec<Arc<PathBuf>>,

    // These are propagated when `filter` on request
    // is `false`
    pub assets: Vec<Arc<PathBuf>>,
    pub partials: Vec<Arc<PathBuf>>,
    pub includes: Vec<Arc<PathBuf>>,
    pub resources: Vec<Arc<PathBuf>>,
    pub locales: Vec<Arc<PathBuf>>,
    pub data_sources: Vec<Arc<PathBuf>>,
    pub short_codes: Vec<Arc<PathBuf>>,

    // TODO: books too!

    // Unrecognized files that should be copied
    pub other: Vec<Arc<PathBuf>>,
}

#[derive(Debug)]
pub struct CollateResult {
    pub inner: Arc<Mutex<CollateInfo>>,
}

impl CollateResult {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(
                CollateInfo {
                    errors: Vec::new(),
                    all: HashMap::new(),
                    pages: Vec::new(),
                    files: Vec::new(),
                    dirs: Vec::new(),

                    assets: Vec::new(),
                    partials: Vec::new(),
                    includes: Vec::new(),
                    resources: Vec::new(),
                    locales: Vec::new(),
                    data_sources: Vec::new(),
                    short_codes: Vec::new(),

                    other: Vec::new(),
                }
            ))
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
                            Ok(page_info) => {
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
                        info.pages.push(Arc::clone(&key));
                    } else {
                        if !req.filter {

                            // TODO: store the layout?
                            if let Some(ref layout) = req.options.settings.layout {
                                if key.starts_with(layout) {
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
