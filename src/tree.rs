use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use serde_json::{json, Value, Map};

use crate::build::loader;
use crate::build::matcher;
use crate::build::context::Context;
use crate::build::matcher::FileType;

use crate::{
    utils,
    Error,
    HTML,
    INDEX_HTML,
    INDEX_STEM,
    MD
};

pub type ItemData = Map<String, Value>;

#[derive(Debug)]
pub struct PathAndHref {
    path: PathBuf,
    href: String,
}

pub struct ListOptions {
    pub sort: bool,
    pub sort_key: String,
    pub dir: String,
    pub depth: usize,
}

pub fn listing<P: AsRef<Path>>(target: P, list: &ListOptions, ctx: &Context) -> Result<Vec<ItemData>, Error> {
    let mut path: PathBuf = target.as_ref().to_path_buf();

    // Resolve using a dir string argument
    if !list.dir.is_empty() {
        // Note that PathBuf.push() with a value of "/"
        // will make the entire path point to "/" and not
        // concatenate the path as expected so we use a
        // string instead
        let mut dir_target = ctx.options.source.to_string_lossy().to_string();
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
        return children(&path, &parent, &list, ctx);
    }

    Ok(vec![])
}

fn children<P: AsRef<Path>>(file: P, parent: &Path, list: &ListOptions, ctx: &Context) -> Result<Vec<ItemData>, Error> {
    let mut entries: Vec<ItemData> = Vec::new();

    let source = &ctx.options.source;
    let target = &ctx.options.target;

    //let p = parent.as_ref();

    let rel_base = parent
        .strip_prefix(source)
        .unwrap_or(Path::new(""));

    for result in WalkBuilder::new(parent).max_depth(Some(list.depth)).build() {
        match result {
            Ok(entry) => {
                let path = entry.path();

                // Prevent duplicate index on /folder/ and /folder/index.md
                if path == parent {
                    continue;
                }

                let mut href = "".to_string();

                // NOTE: there is an invalid lint warning on this
                // NOTE: saying it is not used but we pass it to 
                // NOTE: the json!() macro later
                #[allow(unused_assignments)]
                let mut this: bool = false;

                let mut data: Map<String, Value> = Map::new();

                //println!("children {:?}", path);

                if path.is_file() {

                    this = path == file.as_ref();

                    let extensions = &ctx.config.extensions.as_ref().unwrap();
                    let file_type = matcher::get_type(path, extensions);
                    match file_type {
                        FileType::Markdown | FileType::Template => {
                            let mut dest = matcher::destination(
                                source,
                                target,
                                &path.to_path_buf(),
                                &file_type,
                                extensions,
                                ctx.options.clean_url,
                            )?;
                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                            data = loader::compute(&path);

                        }
                        _ => {}
                    }
                } else {
                    this = path == parent;

                    // For directories try to find a potential index
                    // file and generate a destination
                    let mut dir_index = path.to_path_buf();
                    dir_index.push(INDEX_STEM);
                    let candidates =
                        vec![dir_index.with_extension(MD), dir_index.with_extension(HTML)];

                    for f in candidates {
                        if f.exists() {
                            let extensions = &ctx.config.extensions.as_ref().unwrap();
                            let file_type = matcher::get_type(&f, extensions);
                            let mut dest = matcher::destination(
                                source,
                                target,
                                &f,
                                &file_type,
                                extensions,
                                ctx.options.clean_url,
                            )?;

                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                            data = loader::compute(&f);

                            break;
                        }
                    }
                }

                if utils::is_draft(&data, &ctx.options) {
                    continue
                }

                if !href.is_empty() {
                    if ctx.options.clean_url && !ctx.options.index_links {
                        if href.ends_with(INDEX_HTML) {
                            href.truncate(href.len() - INDEX_HTML.len());
                        }
                    }

                    data.insert("href".to_owned(), json!(href));
                    data.insert("self".to_owned(), json!(this));
                    entries.push(data);
                }
            }
            Err(e) => {
                return Err(Error::from(e))
            }
        }
    }

    if list.sort {
        entries.sort_by(|a,b| {
            let mut s1 = "".to_string();
            let mut s2 = "".to_string();
            if let Some(v1) = a.get(&list.sort_key) {
                if let Some(v2) = b.get(&list.sort_key) {
                    if let Some(v1) = v1.as_str() {
                        if let Some(v2) = v2.as_str() {
                            s1 = v1.to_string();
                            s2 = v2.to_string();
                        }
                    }
                }
            }

            s1.partial_cmp(&s2).unwrap()
        });
    }

    Ok(entries)
}

