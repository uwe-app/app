use std::fs;

use std::process::{Command, Stdio};
use std::path::Path;
use std::path::PathBuf;
use std::fs::Metadata;
use std::convert::AsRef;

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

    fn get_mod_time(&self, _meta: &Metadata) -> &str {
        // TODO: generate modTime
        "time.Now()"
    }

    fn get_dir_entry(&self, name: String, key: &str, _path: &Path, meta: Metadata) -> String {
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

    fn get_file_entry(&self, name: String, key: &str, path: &Path, meta: Metadata) -> Result<String, Error> {
        let mod_time = self.get_mod_time(&meta);
        let content = self.get_file_content(path)?;
        Ok(format!("\"{}\": &AssetFile{{name:\"{}\", modTime: {}, size: {}, content: {}}},\n",
                key,
                name,
                mod_time,
                meta.len(),
                content))
    }

    fn get_dir_start(&self, key: &str) -> String {
        format!("\nfs.assets[\"{}\"].(*DirInfo).entries = []FileInfo{{\n", key)
    }

    fn get_dir_index_entry(&self, key: &str) -> String {
        format!("fs.assets[\"{}\"].(FileInfo),\n", key)
    }

    fn get_dir_end(&self) -> &str {
        "}\n"
    }

    fn get_key(&self, source: &PathBuf, path: &Path) -> Result<(PathBuf, String), Error> {
        let rel = path.strip_prefix(source)?;
        let mut key = "/".to_string();
        key.push_str(&rel.clone().to_string_lossy().into_owned());
        Ok((rel.to_path_buf(), key))
    }

    // NOTE: In order to build up the file index correctly this iterates
    // NOTE: the entire directory tree once to gather all entries and then
    // NOTE: iterates again to build up the directory indices. This is clearly
    // NOTE: inefficient but as we cannot detect when we leave a directory
    // NOTE: using WalkBuilder then there is no way to terminate a directory
    // NOTE: index correctly.
    //
    // NOTE: A future version could improve this with manual iteration that maintains
    // NOTE: a stack of entered directories, removing the WalkBuilder.
    pub fn generate(&self, source: &PathBuf) -> Result<String, Error> {
        let mut s = "".to_owned();

        s.push_str(self.get_file_prefix());

        let mut dirs: Vec<PathBuf> = Vec::new();

        for result in WalkBuilder::new(source).build() {
            match result {
                Ok(entry) => {
                    if let Ok(meta) = entry.metadata() {
                        let path = entry.path();

                        let (rel, key) = self.get_key(source, &path)?;
                        let is_root = rel == Path::new("").to_path_buf();

                        if let Some(name) = path.file_name() {
                            let mut nm = name.to_string_lossy().into_owned();
                            if is_root {
                                nm = "/".to_string();
                            }

                            info!("add {}", &key);

                            if path.is_dir() {
                                dirs.push(path.to_path_buf());
                                s.push_str(&self.get_dir_entry(nm, &key, &path, meta));
                            } else if path.is_file() {
                                s.push_str(&self.get_file_entry(nm, &key, &path, meta)?);
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

        
        s.push_str(self.get_file_suffix());
        s.push_str(self.get_init_prefix());

        for dir in dirs {
            let (_, key) = self.get_key(source, &dir)?;
            info!("index {:?}", key);
            s.push_str(&self.get_dir_start(&key));
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let (_, key) = self.get_key(source, &path)?;
                s.push_str(&self.get_dir_index_entry(&key));
            }
            s.push_str(self.get_dir_end());
        }

        s.push_str(self.get_init_suffix());

        Ok(s)
    }

    pub fn version(&self) -> Result<(), Error> {
        Command::new("go")
            .arg("version")
            .stdout(Stdio::null())
            .spawn()?;
        Ok(()) 
    }

    pub fn compile<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {

        // TODO: custom file name
        // TODO: platform filters

        Command::new("go")
            .current_dir(path)
            .arg("build")
            .spawn()?;

        Ok(()) 
    }
}
