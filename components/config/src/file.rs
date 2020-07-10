use std::path::{Path, PathBuf};
use super::Error;

use super::config::{Config, ExtensionConfig};

use crate::config::{HTML, INDEX_STEM};

#[derive(Debug, Clone)]
pub enum FileType {
    Markdown,
    Template,
    Unknown,
}

#[derive(Debug)]
pub struct FileOptions<'a> {
    // Request a 1:1 output file
    pub exact: bool,
    // Rewrite to directory index.html file
    pub rewrite_index: bool,
    // A base href used to extract sub-directories
    pub base_href: &'a Option<String>,
}

impl Default for FileOptions<'_> {
    fn default() -> Self {
        Self {
            exact: false,
            rewrite_index: false,
            base_href: &None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileInfo<'a> {
    // The root of the source files
    pub config: &'a Config,
    // The root of the source files
    pub source: &'a PathBuf,
    // The root of the build target
    pub target: &'a PathBuf,
    // A source file path
    pub file: &'a PathBuf,
    // The file type
    pub file_type: FileType,
    // An output destination
    pub output: Option<PathBuf>,
}

impl<'a> FileInfo<'a> {
    pub fn new(
        config: &'a Config,
        source: &'a PathBuf,
        target: &'a PathBuf,
        file: &'a PathBuf) -> Self {
        let file_type = FileInfo::get_type(file,config);
        Self {config, source, target, file, file_type, output: None}
    }

    fn has_parse_file_match<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> bool {
        let path = file.as_ref();
        let mut copy = path.to_path_buf();
        for ext in extensions.render.iter() {
            copy.set_extension(ext);
            if copy.exists() {
                return true;
            }
        }
        false
    }

    fn rewrite_index_file<P: AsRef<Path>>(file: P, result: P, extensions: &ExtensionConfig) -> Option<PathBuf> {
        let clean_target = file.as_ref();
        if !FileInfo::is_index(&clean_target) {
            if let Some(parent) = clean_target.parent() {
                if let Some(stem) = clean_target.file_stem() {
                    let mut target = parent.to_path_buf();
                    target.push(stem);
                    target.push(INDEX_STEM);

                    if !FileInfo::has_parse_file_match(&target, extensions) {
                        let clean_result = result.as_ref().clone();
                        if let Some(parent) = clean_result.parent() {
                            if let Some(stem) = clean_result.file_stem() {
                                let mut res = parent.to_path_buf();
                                res.push(stem);
                                res.push(INDEX_STEM);
                                res.set_extension(HTML);
                                return Some(res);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn is_clean<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> bool {
        let target = file.as_ref().to_path_buf();
        let result = target.clone();
        return FileInfo::rewrite_index_file(target, result, extensions).is_some();
    }

    pub fn is_page<P: AsRef<Path>>(p: P, config: &Config) -> bool {
        match FileInfo::get_type(p, config) {
            FileType::Markdown | FileType::Template => {
                true
            },
            _ => false
        }
    }

    pub fn relative_to<P: AsRef<Path>>(file: P, base: P, target: P) -> Result<PathBuf, Error> {
        let f = file.as_ref().canonicalize()?;
        let b = base.as_ref().canonicalize()?;
        let mut t = target.as_ref().to_path_buf();
        let relative = f.strip_prefix(b)?;
        t.push(relative);
        Ok(t)
    }

    pub fn get_type<P: AsRef<Path>>(p: P, config: &Config) -> FileType {
        let extensions = &config.extension.as_ref().unwrap();
        let file = p.as_ref();
        if let Some(ext) = file.extension() {
            let ext = ext.to_string_lossy().into_owned();
            if extensions.render.contains(&ext) {
                if extensions.markdown.contains(&ext) {
                    return FileType::Markdown;
                } else {
                    return FileType::Template;
                }
            }
        }
        FileType::Unknown
    }


    pub fn is_index<P: AsRef<Path>>(file: P) -> bool {
        if let Some(nm) = file.as_ref().file_stem() {
            if nm == INDEX_STEM {
                return true;
            }
        }
        false
    }

    // Build the output file path.
    // 
    // Does not modify the file extension, rewrite the index of change the slug,
    // this is used when we copy over files with a direct 1:1 correlation.
    //
    fn output(&self, options: &FileOptions) -> Result<PathBuf, Error> {
        let pth = self.file.clone();

        // NOTE: When watching files we can get absolute
        // NOTE: paths passed for `file` even when `source`
        // NOTE: is relative. This handles that case by making
        // NOTE: the `source` absolute based on the current working
        // NOTE: directory.
        let mut src: PathBuf = self.source.clone();
        if pth.is_absolute() && src.is_relative() {
            if let Ok(cwd) = std::env::current_dir() {
                src = cwd.clone();
                src.push(self.source);
            }
        }

        let mut relative = pth.strip_prefix(src)?;
        if let Some(ref base) = options.base_href {
            if relative.starts_with(base) {
                relative = relative.strip_prefix(base)?;
            }
        }

        let result = self.target.clone().join(relative);
        return Ok(result);
    }

    // Build the destination file path and update the file extension.
    pub fn destination(&mut self, config: &Config, options: &FileOptions) -> Result<(), Error> {
        let pth = self.file.clone();
        let mut result = self.output(options)?;
        if !options.exact {
            match self.file_type {
                FileType::Markdown | FileType::Template => {
                    let extensions = &config.extension.as_ref().unwrap();
                    if let Some(ext) = pth.extension() {
                        let ext = ext.to_string_lossy().into_owned();
                        for (k, v) in &extensions.map {
                            if ext == *k {
                                result.set_extension(v);
                                break;
                            }
                        }
                    }

                    if options.rewrite_index {
                        if let Some(res) = FileInfo::rewrite_index_file(pth.as_path(), result.as_path(), extensions) {
                            result = res;
                        }
                    }
                }
                _ => {}
            }
        }

        self.output = Some(result);
        return Ok(());
    }
}

