use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use serde_json::{json, Value, Map};

use crate::{
    matcher,
    DataLoader,
    FileType,
    Error,
    Options,
    HTML,
    INDEX_HTML,
    INDEX_STEM,
    MD
};

pub type ItemData = Map<String, Value>;

pub struct ListOptions {
    pub sort: bool,
    pub dir: String,
}

pub fn listing<P: AsRef<Path>>(target: P, list: &ListOptions, opts: &Options) -> Result<Vec<ItemData>, Error> {
    let mut path: PathBuf = target.as_ref().to_path_buf();

    // Resolve using a dir string argument
    if !list.dir.is_empty() {
        // Note that PathBuf.push() with a value of "/"
        // will make the entire path point to "/" and not
        // concatenate the path as expected so we use a
        // string instead
        let mut dir_target = opts.source.to_string_lossy().to_string();
        dir_target.push_str(&list.dir);

        let dir_dest = Path::new(&dir_target);
        if !dir_dest.exists() || !dir_dest.is_dir() {
            return Err(Error::new("Path parameter for listing does not resolve to a directory".to_string()));
        }

        // Later we find the parent so this makes it consistent
        // with using a file as the input path
        dir_target.push_str(INDEX_HTML);
        path = PathBuf::from(dir_target);
    }

    if let Some(parent) = path.parent() {
        //parent.foo();
        return children(&path, &parent, &opts);
    }

    Ok(vec![])
}

fn children<P: AsRef<Path>>(file: P, parent: &Path, opts: &Options) -> Result<Vec<ItemData>, Error> {
    let mut entries: Vec<ItemData> = Vec::new();

    let source = &opts.source;
    let target = &opts.target;

    //let p = parent.as_ref();

    let rel_base = parent
        .strip_prefix(source)
        .unwrap_or(Path::new(""));

    let loader = DataLoader::new(opts);

    for result in WalkBuilder::new(parent).max_depth(Some(1)).build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                let mut href = "".to_string();
                let mut data = DataLoader::create();

                if path.is_file() {
                    // Ignore self
                    if path == file.as_ref() {
                        continue;
                    }

                    let file_type = matcher::get_type(path);
                    match file_type {
                        FileType::Markdown | FileType::Html => {
                            let mut dest = matcher::destination(
                                source,
                                target,
                                &path.to_path_buf(),
                                &file_type,
                                opts.clean_url,
                            );
                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                            //loader.load(&path, &mut data);

                            if let Err(e) = loader.load(&path, &mut data) {
                                return Err(e)
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Ignore self
                    if path == parent {
                        continue;
                    }

                    // For directories try to find a potential index
                    // file and generate a destination
                    let mut dir_index = path.to_path_buf();
                    dir_index.push(INDEX_STEM);
                    let candidates =
                        vec![dir_index.with_extension(MD), dir_index.with_extension(HTML)];

                    for f in candidates {
                        if f.exists() {
                            let file_type = matcher::get_type(&f);
                            let mut dest = matcher::destination(
                                source,
                                target,
                                &f,
                                &file_type,
                                opts.clean_url,
                            );

                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                            if let Err(e) = loader.load(&f, &mut data) {
                                return Err(e)
                            }

                        }
                    }
                }

                if !href.is_empty() {
                    if opts.clean_url {
                        if href.ends_with(INDEX_HTML) {
                            href.truncate(href.len() - INDEX_HTML.len());
                        }
                    }
                    data.insert("href".to_owned(), json!(href));
                    entries.push(data);
                }
            }
            Err(e) => {
                return Err(Error::from(e))
            }
        }
    }

    Ok(entries)
}

