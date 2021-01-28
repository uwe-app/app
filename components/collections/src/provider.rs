use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use futures::{future, stream, Stream, StreamExt, TryStreamExt};
use tokio::fs::{self, DirEntry};

use collator::CollateInfo;
use config::indexer::{DataProvider, SourceProvider, SourceType};
use config::{Config, RuntimeOptions};

use super::identifier::{ComputeIdentifier, Strategy};
use super::{Error, Result};

static JSON: &str = "json";

#[derive(thiserror::Error, Debug)]
pub enum DeserializeError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

type ProviderResult = std::result::Result<Value, DeserializeError>;

pub struct LoadRequest<'a> {
    pub source: &'a PathBuf,
    pub config: &'a Config,
    pub options: &'a RuntimeOptions,
    pub collation: &'a CollateInfo,
    pub definition: &'a DataProvider,
    pub strategy: Strategy,
    pub kind: &'a SourceType,
    pub provider: &'a SourceProvider,
}

fn find_recursive(
    path: impl Into<PathBuf>,
) -> impl Stream<Item = io::Result<DirEntry>> + Send + 'static {
    async fn one_level(
        path: PathBuf,
        to_visit: &mut Vec<PathBuf>,
    ) -> io::Result<Vec<DirEntry>> {
        let mut dir = fs::read_dir(path).await?;
        let mut files = Vec::new();
        while let Some(child) = dir.next_entry().await? {
            if child.metadata().await?.is_dir() {
                to_visit.push(child.path());
            } else {
                files.push(child)
            }
        }

        Ok(files)
    }

    stream::unfold(vec![path.into()], |mut to_visit| async {
        let path = to_visit.pop()?;
        let file_stream = match one_level(path, &mut to_visit).await {
            Ok(files) => stream::iter(files).map(Ok).left_stream(),
            Err(e) => stream::once(async { Err(e) }).right_stream(),
        };

        Some((file_stream, to_visit))
    })
    .flatten()
}

pub struct Provider {}

impl Provider {
    fn deserialize<S: AsRef<str>>(
        kind: &SourceType,
        content: S,
    ) -> ProviderResult {
        match kind {
            SourceType::Json => Ok(serde_json::from_str(content.as_ref())?),
            SourceType::Toml => Ok(toml::from_str(content.as_ref())?),
        }
    }

    pub async fn load(
        req: LoadRequest<'_>,
    ) -> Result<BTreeMap<String, Arc<Value>>> {
        match req.provider {
            SourceProvider::Files => Provider::load_files(req).await,
            SourceProvider::Pages => Provider::load_pages(req).await,
        }
    }

    async fn load_pages(
        req: LoadRequest<'_>,
    ) -> Result<BTreeMap<String, Arc<Value>>> {
        let mut docs: BTreeMap<String, Arc<Value>> = BTreeMap::new();
        let limit: usize = 100;

        stream::iter(req.collation.pages())
            .filter(|(p, _)| {
                if !p.starts_with(req.source) {
                    return future::ready(false);
                }
                if !req.definition.matcher().is_empty() {
                    if let Some(relative) = p.strip_prefix(req.source).ok() {
                        if req.definition.matcher().is_excluded(&relative) {
                            return future::ready(false);
                        }
                    }
                }
                future::ready(true)
            })
            .enumerate()
            .map(Ok)
            .try_for_each_concurrent(limit, |(count, (path, _))| {
                // Convert the page data to a Value for indexing
                let data = req.collation.resolve(path).unwrap();
                let page = data.read().unwrap();

                // Must ignore synthetic pages otherwise during 
                // live reload they can be added to a collection 
                // database when it is invalidated which breaks the 
                // logic in various ways and can cause errors with 
                // links not being found due to inclusion of feeds 
                // and other synthetic pages.
                if page.is_synthetic() {
                    return future::ok(())
                }

                let result = serde_json::to_value(&*page);
                match result {
                    Ok(document) => {
                        let key = ComputeIdentifier::id(
                            &Strategy::Count,
                            &path,
                            &document,
                            &count,
                        );
                        if docs.contains_key(&key) {
                            return future::err(Error::DuplicateId {
                                key,
                                path: path.to_path_buf(),
                            });
                        }

                        docs.insert(key, Arc::new(document));
                    }
                    Err(e) => return future::err(Error::from(e)),
                }
                future::ok(())
            })
            .await?;

        Ok(docs)
    }

    async fn load_files(
        req: LoadRequest<'_>,
    ) -> Result<BTreeMap<String, Arc<Value>>> {
        let mut docs: BTreeMap<String, Arc<Value>> = BTreeMap::new();
        let limit: usize = 100;

        Provider::find_documents(&req)
            .try_for_each_concurrent(limit, |(count, entry)| {
                let path = entry.path();
                let result = utils::fs::read_string(&path);
                match result {
                    Ok(content) => {
                        let result = Provider::deserialize(&req.kind, &content);
                        match result {
                            Ok(document) => {
                                let key = ComputeIdentifier::id(
                                    &req.strategy,
                                    &path,
                                    &document,
                                    &count,
                                );
                                if docs.contains_key(&key) {
                                    return future::err(Error::DuplicateId {
                                        key,
                                        path: path.to_path_buf(),
                                    });
                                }

                                docs.insert(key, Arc::new(document));
                            }
                            Err(e) => return future::err(Error::from(e)),
                        }
                    }
                    Err(e) => return future::err(Error::from(e)),
                }

                future::ok(())
            })
            .await?;
        Ok(docs)
    }

    fn find_documents<'a>(
        req: &'a LoadRequest<'a>,
    ) -> Pin<
        Box<
            dyn Stream<Item = std::result::Result<(usize, DirEntry), Error>>
                + 'a,
        >,
    > {
        find_recursive(&req.source)
            .map_err(Error::from)
            .filter(|result| {
                if let Ok(entry) = result {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        return future::ready(ext == JSON);
                    }
                }
                future::ready(false)
            })
            .enumerate()
            .map(|(c, r)| Ok((c, r.unwrap())))
            .boxed()
    }
}
