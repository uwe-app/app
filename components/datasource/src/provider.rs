use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::result::{Result as StdResult};

use serde_json::Value;
use serde::{Deserialize, Serialize};
use ignore::{WalkBuilder, WalkState};

use tokio::fs::{self, DirEntry};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};

use config::Config;

use super::{Result, Error};
use super::identifier::DocumentIdentifier;

static JSON: &str = "json";

#[derive(thiserror::Error, Debug)]
pub enum DeserializeError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

type ProviderResult = std::result::Result<Value, DeserializeError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SourceType {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "toml")]
    Toml,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SourceProvider {
    #[serde(rename = "documents")]
    Documents,
    #[serde(rename = "pages")]
    Pages,
}

pub struct LoadRequest<'a> {
    pub source: &'a PathBuf,
    pub config: &'a Config,
    pub id: Box<dyn DocumentIdentifier + 'a>,
    pub documents: PathBuf,
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

    pub fn load(req: LoadRequest) -> Result<BTreeMap<String, Value>> {
        match req.provider {
            SourceProvider::Documents => {
                Provider::load_documents(req)
            },
            SourceProvider::Pages => {
                Provider::load_pages(req)
            }
        }
    }

    #[tokio::main]
    async fn load_pages(req: LoadRequest) -> Result<BTreeMap<String, Value>> {
        let mut docs: BTreeMap<String, Value> = BTreeMap::new();

        let filters = matcher::get_filters(req.source, req.config);
        let (tx, rx) = flume::unbounded();

        let extensions = req.config.extension.as_ref().unwrap().clone();

        // We use the walk builder so we can respect the way the compiler
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
                        if path.is_file() && matcher::is_page(&path, &extensions) {
                            println!("Pages walker for file {:?}", path);
                            let _ = tx.send(path.to_path_buf());
                            //docs.insert("foo".to_string(),serde_json::json!(""));
                        }
                    }
                    WalkState::Continue
                }
            )
        });

        let paths = &rx.drain().collect::<Vec<_>>();

        //paths.foo();

        Ok(docs)
    }

    #[tokio::main]
    async fn load_documents(req: LoadRequest) -> Result<BTreeMap<String, Value>> {
        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
        Provider::find_documents(&req)
            .try_for_each(|entry| {
                let path = entry.path();
                let result = utils::fs::read_string(&path);
                match result {
                    Ok(content) => {
                        let result = Provider::deserialize(&req.kind, &content);
                        match result {
                            Ok(document) => {
                                let key = req.id.identifier(&path, &document);
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
        -> Pin<Box<dyn Stream<Item = StdResult<DirEntry, Error>> + 'a>> {

        find_recursive(&req.documents)
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
            .boxed()
    }
}
