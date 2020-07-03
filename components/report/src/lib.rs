use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use thiserror::Error;

use utils;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    #[error(transparent)]
    Ignore(#[from] ignore::Error),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ResultFile {
    pub key: Option<String>,
    pub e_tag: Option<String>,
}

#[derive(Debug)]
pub struct FileBuilder {
    // A base path all files must be relative to
    pub base: PathBuf,
    // When specified this prefix is appended before the path
    pub prefix: Option<String>,
    // List of file keys
    pub keys: HashSet<String>,
}

impl FileBuilder {
    pub fn new(base: PathBuf, prefix: Option<String>) -> Self {
        Self {
            base,
            prefix,
            keys: HashSet::new(),
        }
    }

    fn add<P: AsRef<Path>>(&mut self, raw: P) -> Result<(), Error> {
        let mut key = raw.as_ref().strip_prefix(&self.base)?.to_path_buf();
        key = if let Some(ref prefix) = self.prefix {
            let mut tmp = PathBuf::from(prefix);
            tmp.push(key);
            tmp
        } else {
            key
        };

        let key_str = key.to_string_lossy().into_owned();
        // Assuming we will compare with s3 using a slash as the folder delimiter
        self.keys.insert(utils::url::to_href_separator(key_str));
        Ok(())
    }

    pub fn from_key<S: AsRef<str>>(&self, key: S) -> PathBuf {
        let mut pth = self.base.clone();

        if let Some(ref prefix) = self.prefix {
            let mut tmp = key.as_ref().trim_start_matches(prefix);
            tmp = tmp.trim_start_matches("/");
            pth.push(tmp);
        } else {
            pth.push(key.as_ref());
        }

        pth
    }

    pub fn walk(&mut self) -> Result<(), Error> {
        for result in WalkBuilder::new(&self.base).follow_links(true).build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        self.add(&path)?;
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
        Ok(())
    }
}

//#[cfg(test)]
//mod tests {
//#[test]
//fn it_works() {
//assert_eq!(2 + 2, 4);
//}
//}
