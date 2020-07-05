use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;

use serde_json::Value;
use tokio::fs::{self, DirEntry};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};

use super::{Result, Error};
use super::identifier::DocumentIdentifier;

static JSON: &str = "json";

pub fn find_files(path: impl Into<PathBuf>) -> impl Stream<Item = io::Result<DirEntry>> + Send + 'static {

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

pub struct LoadRequest {
    pub id: Box<dyn DocumentIdentifier + 'static>,
    pub documents: PathBuf,
}

pub struct DocumentsLoader {}

impl DocumentsLoader {

    #[tokio::main]
    pub async fn load(req: LoadRequest) -> Result<BTreeMap<String, Value>> {
        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
        find_files(&req.documents)
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
            .try_for_each(|entry| {
                let path = entry.path();
                let result = utils::fs::read_string(&path);
                match result {
                    Ok(content) => {
                        let result: std::result::Result<Value, serde_json::Error> = serde_json::from_str(&content);
                        match result {
                            Ok(document) => {
                                let key = req.id.identifier(&path, &document);

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

        //println!("Returning the result {:?}", docs);
        Ok(docs)
    }
}

