use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::result::{Result as StdResult};

use serde_json::Value;
use ignore::{WalkBuilder, WalkState};

use tokio::fs::{self, DirEntry};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};

use config::{Config, FileInfo, RuntimeOptions};
use config::indexer::{SourceType, SourceProvider};

use super::{Result, Error};
use super::identifier::{ComputeIdentifier, Strategy};

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
    pub strategy: Strategy,
    pub kind: SourceType,
    pub provider: SourceProvider,
}

fn find_recursive(path: impl Into<PathBuf>) -> impl Stream<Item = io::Result<DirEntry>> + Send + 'static {
    async fn one_level(path: PathBuf, to_visit: &mut Vec<PathBuf>) -> io::Result<Vec<DirEntry>> {
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

    stream::unfold(vec![path.into()], |mut to_visit| {
        async {
            let path = to_visit.pop()?;
            let file_stream = match one_level(path, &mut to_visit).await {
                Ok(files) => stream::iter(files).map(Ok).left_stream(),
                Err(e) => stream::once(async { Err(e) }).right_stream(),
            };

            Some((file_stream, to_visit))
        }
    })
    .flatten()
}

pub struct Provider {}

impl Provider {

    fn deserialize<S: AsRef<str>>(kind: &SourceType, content: S) -> ProviderResult {
        match kind {
            SourceType::Json => {
                Ok(serde_json::from_str(content.as_ref())?)
            },
            SourceType::Toml => {
                Ok(toml::from_str(content.as_ref())?)
            }
        }  
    }

    pub async fn load(req: LoadRequest<'_>) -> Result<BTreeMap<String, Value>> {
        match req.provider {
            SourceProvider::Documents => {
                Provider::load_documents(req).await
            },
            SourceProvider::Pages => {
                Provider::load_pages(req).await
            }
        }
    }

    async fn load_pages(req: LoadRequest<'_>) -> Result<BTreeMap<String, Value>> {
        let filters = config::filter::get_filters(req.options, req.config);
        let (tx, rx) = flume::unbounded();

        // Use the walk builder to respect the way the compiler
        // ignores and filters files
        WalkBuilder::new(req.source)
            .filter_entry(move |e| {
                let path = e.path();
                if filters.contains(&path.to_path_buf()) {
                    return false;
                }
                true
            })
            .build_parallel()
            .run(|| {
                Box::new(|result| {
                    let tx = tx.clone();
                    if let Ok(entry) = result {
                        let path = entry.path();
                        if path.is_file() && FileInfo::is_page(&path, req.options) {
                            let _ = tx.send(path.to_path_buf());
                        }
                    }
                    WalkState::Continue
                }
            )
        });

        Provider::compute_pages(req, rx.drain().collect::<Vec<_>>()).await
    }

    async fn compute_pages(req: LoadRequest<'_>, paths: Vec<PathBuf>) -> Result<BTreeMap<String, Value>> {
        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
        let limit: usize = 100;

        stream::iter(paths)
            .enumerate()
            .map(Ok)
            .try_for_each_concurrent(limit, |(count, path)| {
                let result = loader::compute(&path, req.config, req.options, true);
                match result {
                    Ok(data) => {
                        let result = serde_json::to_value(data);
                        match result {
                            Ok(document) => {
                                let key = ComputeIdentifier::id(
                                    &Strategy::Count, &path, &document, &count);
                                if docs.contains_key(&key) {
                                    return future::err(Error::DuplicateId {key, path});
                                }
                                docs.insert(key, document);
                            },
                            Err(e) => {
                                return future::err(Error::from(e))
                            }
                        }
                    },
                    Err(e) => {
                        return future::err(Error::from(e))
                    }
                }

                future::ok(())
            }).await?;
        Ok(docs)
    }

    async fn load_documents(req: LoadRequest<'_>) -> Result<BTreeMap<String, Value>> {
        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
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
                                let key = ComputeIdentifier::id(&req.strategy, &path, &document, &count);
                                if docs.contains_key(&key) {
                                    return future::err(Error::DuplicateId {key, path: path.to_path_buf()});
                                }
                                docs.insert(key, document);
                            },
                            Err(e) => {
                                return future::err(Error::from(e))
                            }
                        }
                    },
                    Err(e) => {
                        return future::err(Error::from(e))
                    }
                }

                future::ok(())
            }).await?;
        Ok(docs)
    }

    fn find_documents<'a>(req: &'a LoadRequest<'a>)
        -> Pin<Box<dyn Stream<Item = StdResult<(usize, DirEntry), Error>> + 'a>> {

        find_recursive(&req.source)
            .map_err(Error::from)
            .filter(|result| {
                if let Ok(entry) = result {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        return future::ready(ext == JSON)
                    }
                }
                future::ready(false)
            })
            .enumerate()
            .map(|(c, r)| Ok((c, r.unwrap())))
            .boxed()
    }
}
