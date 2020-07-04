use std::convert::AsRef;
use std::fs::{self, Metadata};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::SystemTime;

use human_bytes::human_bytes;
use ignore::WalkBuilder;
use log::info;

use utils;

use crate::BundleError as Error;

pub enum Platform {
    Linux(String),
    Darwin(String),
    Windows(String),
}

impl Platform {
    pub fn linux() -> Self {
        Platform::Linux(String::from("linux"))
    }

    pub fn darwin() -> Self {
        Platform::Darwin(String::from("darwin"))
    }

    pub fn windows() -> Self {
        Platform::Windows(String::from("windows"))
    }

    pub fn to_string(&self) -> &String {
        match *self {
            Platform::Linux(ref s) => return s,
            Platform::Darwin(ref s) => return s,
            Platform::Windows(ref s) => return s,
        }
    }
}

pub enum Arch {
    Amd64(String),
}

impl Arch {
    pub fn amd64() -> Self {
        Arch::Amd64(String::from("amd64"))
    }

    pub fn to_string(&self) -> &String {
        match *self {
            Arch::Amd64(ref s) => return s,
        }
    }
}

pub struct Target {
    pub platform: Platform,
    pub arch: Arch,
}

impl Target {
    pub fn get_binary_name(&self, name: &str) -> String {
        match self.platform {
            Platform::Linux(ref s) | Platform::Darwin(ref s) => return format!("{}-{}", name, s),
            Platform::Windows(ref s) => return format!("{}-{}.exe", name, s),
        }
    }
}

pub struct Bundler;

impl Bundler {
    pub fn new() -> Self {
        Bundler {}
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
        // FIXME: generate modTime
        "time.Now()"
    }

    fn get_dir_entry(&self, name: String, key: &str, _path: &Path, meta: Metadata) -> String {
        let mod_time = self.get_mod_time(&meta);
        format!(
            "\"{}\": &DirInfo{{name:\"{}\", modTime: {}}},\n",
            key, name, mod_time
        )
    }

    fn get_file_content(&self, path: &Path) -> Result<String, Error> {
        let mut s = "".to_owned();
        s.push_str("[]byte(\"");
        let data = utils::fs::read_bytes(path)?;
        for b in data {
            s.push_str(&format!("\\x{:02x}", b));
        }
        s.push_str("\"),");
        Ok(s)
    }

    fn get_file_entry(
        &self,
        name: String,
        key: &str,
        path: &Path,
        meta: Metadata,
    ) -> Result<String, Error> {
        let mod_time = self.get_mod_time(&meta);
        let content = self.get_file_content(path)?;
        Ok(format!(
            "\"{}\": &AssetFile{{name:\"{}\", modTime: {}, size: {}, content: {}}},\n",
            key,
            name,
            mod_time,
            meta.len(),
            content
        ))
    }

    fn get_dir_start(&self, key: &str) -> String {
        format!(
            "\nfs.assets[\"{}\"].(*DirInfo).entries = []FileInfo{{\n",
            key
        )
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
    // NOTE: a stack of entered directories and remove the WalkBuilder.
    pub fn generate(&self, source: &PathBuf) -> Result<String, Error> {
        let mut s = "".to_owned();

        s.push_str(self.get_file_prefix());

        let mut dirs: Vec<PathBuf> = Vec::new();

        for result in WalkBuilder::new(source).follow_links(true).build() {
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
                                return Err(Error::UnknownPathType);
                            }
                        } else {
                            return Err(Error::NoFileName);
                        }
                    } else {
                        return Err(Error::NoFileMetaData);
                    }
                }
                Err(e) => return Err(Error::from(e)),
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
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with(".") {
                        continue;
                    }
                }
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

    pub fn compile<P: AsRef<Path>>(
        &self,
        path: P,
        name: &str,
        targets: Vec<Target>,
    ) -> Result<Vec<PathBuf>, Error> {
        let mut result: Vec<PathBuf> = Vec::new();
        info!("compile {}", path.as_ref().display());
        for target in targets {
            let name = target.get_binary_name(name);
            let mut dest = path.as_ref().to_path_buf();
            dest.push(&name);
            info!(
                "{} ({} {})",
                &name,
                target.platform.to_string(),
                target.arch.to_string()
            );
            let now = SystemTime::now();
            Command::new("go")
                .current_dir(path.as_ref())
                .env("GOOS", target.platform.to_string())
                .env("GOARCH", target.arch.to_string())
                .arg("build")
                .arg("-o")
                .arg(&name)
                .output()?;
            if let Ok(t) = now.elapsed() {
                if let Ok(meta) = dest.metadata() {
                    let bytes = human_bytes(meta.len() as f64);
                    info!("{} {} in {:?}", &name, bytes, t);
                    result.push(dest);
                }
            }
        }
        Ok(result)
    }
}
