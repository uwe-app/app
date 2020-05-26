use std::path::Path;
use std::path::PathBuf;
use std::fs::Metadata;

use ignore::WalkBuilder;

use crate::Error;
use crate::utils;

use log::info;

pub struct Bundler;

impl Bundler {
    pub fn new() -> Self {
        Bundler{}
    }

    fn get_file_prefix(&self) -> &str {
        "package main
import (
	\"os\"
	\"time\"
)
type FileInfo = os.FileInfo
var fs = &EmbeddedFileSystem{assets: AssetMap {\n" 
    }

    fn get_file_suffix(&self) -> &str {
        "}};\n" 
    }

    fn get_init_prefix(&self) -> &str {
        "func init () {"
    }

    fn get_init_suffix(&self) -> &str {
        "}"
    }

    fn get_mod_time(&self, meta: &Metadata) -> &str {
        // TODO: generate modTime
        "time.Now()"
    }

    fn get_dir_entry(&self, rel: &Path, name: String, _path: &Path, meta: Metadata) -> String {
        let key = rel.display();
        let mod_time = self.get_mod_time(&meta);
        format!("\"{}\": &DirInfo{{name:\"{}\", modTime: {}}},\n", key, name, mod_time)
    }

    fn get_file_content(&self, path: &Path) -> Result<String, Error> {
        let mut s = "".to_owned();
        s.push_str("[]byte(\"");
        let data = utils::read_bytes(path)?;
        for b in data {
            s.push_str(&format!("\\x{:02x}", b));
        }
        s.push_str("\"),");
        Ok(s)
    }

    fn get_file_entry(&self, rel: &Path, name: String, path: &Path, meta: Metadata) -> Result<String, Error> {
        let key = rel.display();
        let mod_time = self.get_mod_time(&meta);
        let content = self.get_file_content(path)?;
        Ok(format!("\"{}\": &AssetFile{{name:\"{}\", modTime: {}, size: {}, content: {}}},\n",
                key,
                name,
                mod_time,
                meta.len(),
                content))
    }

    pub fn generate(&self, source: &PathBuf) -> Result<String, Error> {
        let mut s = "".to_owned();
        s.push_str(self.get_file_prefix());

        // TODO: insert files
        
        for result in WalkBuilder::new(source).build() {
            match result {
                Ok(entry) => {
                    if let Ok(meta) = entry.metadata() {
                        let path = entry.path();

                        let mut rel = path.strip_prefix(source)?;
                        let is_root = rel == Path::new("");
                        if is_root {
                            rel = Path::new("/");
                        }

                        if let Some(name) = path.file_name() {
                            let mut nm = name.to_string_lossy().into_owned();
                            if is_root {
                                nm = "/".to_string();
                            }

                            info!("add {}", rel.display());

                            if path.is_dir() {
                                //println!("Add dir {}", rel.display());
                                s.push_str(&self.get_dir_entry(&rel, nm, &path, meta));
                            } else if path.is_file() {
                                s.push_str(&self.get_file_entry(&rel, nm, &path, meta)?);
                                //println!("Add file {}", rel.display());
                            } else {
                                return Err(Error::new("unknown path type encountered".to_string()))
                            }
                        } else {
                            return Err(Error::new("failed to determine file name".to_string()))
                        }
                    } else {
                        return Err(Error::new("failed to get file meta data".to_string()))
                    }

                },
                Err(e) => return Err(Error::from(e))
            }
        }
        //
        s.push_str(self.get_file_suffix());

        s.push_str(self.get_init_prefix());

        println!("TODO: map dir entries");

        // TODO: map directory entries
        s.push_str(self.get_init_suffix());

        Ok(s)
    }
}
