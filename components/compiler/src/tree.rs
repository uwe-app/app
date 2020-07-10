use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use serde_json::json;

use config::{Page, FileInfo, FileType, FileOptions};

use super::context::Context;
use crate::{Error, HTML, INDEX_HTML, INDEX_STEM, MD};

pub type ItemData = Page;

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

pub fn listing<P: AsRef<Path>>(
    target: P,
    list: &ListOptions,
    ctx: &Context,
) -> Result<Vec<ItemData>, Error> {

    let mut path = target.as_ref().to_path_buf();

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
            return Err(Error::ListingNotDirectory(dir_dest.to_path_buf()));
        }

        // Later we find the parent so this makes it consistent
        // with using a file as the input path
        dir_target.push_str(INDEX_HTML);
        path = PathBuf::from(dir_target);
    }

    if let Some(parent) = path.parent() {
        return children(&path, &parent, &list, ctx);
    }

    Ok(vec![])
}

fn children<P: AsRef<Path>>(
    file: P,
    parent: &Path,
    list: &ListOptions,
    ctx: &Context,
) -> Result<Vec<ItemData>, Error> {
    let mut entries: Vec<ItemData> = Vec::new();

    let source = &ctx.options.source;
    let target = &ctx.options.target;

    let rel_base = parent
        .strip_prefix(source)
        .unwrap_or(Path::new(""));

    for result in WalkBuilder::new(parent)
        .max_depth(Some(list.depth))
        .build() {

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
                let mut this = false;

                let mut data: Page = Default::default();

                //println!("children {:?}", path);

                if path.is_file() {
                    this = path == file.as_ref();

                    let file_type = FileInfo::get_type(path, &ctx.config);
                    match file_type {
                        FileType::Markdown | FileType::Template => {

                            let source_file = path.to_path_buf();
                            let info = FileInfo::new(
                                source,
                                target,
                                &source_file,
                            );

                            let file_opts = FileOptions {
                                file_type: &file_type, 
                                rewrite_index: ctx.options.rewrite_index,
                                ..Default::default()
                            };

                            let mut dest = info.destination(&ctx.config, &file_opts)?;
                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().into();
                            data = loader::compute(&path, &ctx.config, true)?;
                        }
                        _ => {}
                    }
                } else {
                    this = path == parent;

                    // For directories try to find a potential index
                    // file and generate a destination
                    let mut dir_index = path.to_path_buf();
                    dir_index.push(INDEX_STEM);

                    // FIXME: use list of extrensions?
                    let candidates =
                        vec![dir_index.with_extension(MD), dir_index.with_extension(HTML)];

                    for f in candidates {
                        if f.exists() {
                            let file_type = FileInfo::get_type(&f, &ctx.config);
                            let info = FileInfo::new(
                                source,
                                target,
                                &f,
                            );

                            let file_opts = FileOptions {
                                file_type: &file_type, 
                                rewrite_index: ctx.options.rewrite_index,
                                ..Default::default()
                            };

                            let mut dest = info.destination(&ctx.config, &file_opts)?;

                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                            data = loader::compute(&f, &ctx.config, true)?;

                            break;
                        }
                    }
                }

                if super::draft::is_draft(&data, &ctx.options) {
                    continue;
                }

                if !href.is_empty() {
                    if ctx.options.rewrite_index && !ctx.options.include_index {
                        if href.ends_with(INDEX_HTML) {
                            href.truncate(href.len() - INDEX_HTML.len());
                        }
                    }

                    // NOTE: must override the formal href
                    data.href = Some(href);

                    data.extra.insert("self".to_owned(), json!(this));
                    entries.push(data);
                }
            }
            Err(e) => return Err(Error::from(e)),
        }
    }

    if list.sort {
        entries.sort_by(|a, b| {
            let mut s1 = "";
            let mut s2 = "";
            if list.sort_key == "title" {
                s1 = a.title.as_ref().map(|x| &**x).unwrap_or("");
                s2 = b.title.as_ref().map(|x| &**x).unwrap_or("");
            }
            s1.partial_cmp(s2).unwrap()
        });
    }

    Ok(entries)
}
