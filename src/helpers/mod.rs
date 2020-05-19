use std::io;
use std::path::Path;
use std::collections::BTreeMap;
use std::convert::AsRef;

use ignore::WalkBuilder;

use handlebars::*;

use serde_json::value::Value;

use super::matcher;
use super::matcher::FileType;

#[derive(Debug)]
struct TocEntry {
    href: String,
}

fn get_files<P: AsRef<Path>>(file: P, parent: P, ctx: &Value) -> io::Result<Vec<TocEntry>> {

    let mut entries: Vec<TocEntry> = Vec::new();

    let src = ctx.get("source").unwrap().as_str().unwrap();
    let tgt = ctx.get("target").unwrap().as_str().unwrap();
    let layout = ctx.get("layout").unwrap().as_str().unwrap();
    let clean_url = ctx.get("clean_url").unwrap().as_bool().unwrap();
    let source = Path::new(src);
    let target = Path::new(tgt);

    let rel_base = parent.as_ref().strip_prefix(source).unwrap_or(Path::new(""));

    println!("parent {:?}", rel_base);

    for result in WalkBuilder::new(parent.as_ref()).max_depth(Some(1)).build() {

        match result {
            Ok(entry) => {
                let path = entry.path();
                let mut href = "".to_string();

                if path.is_file() {

                    // Ignore self
                    if path == file.as_ref() {
                        continue;
                    }


                    let file_type = matcher::get_type(layout, path);
                    match file_type {
                        FileType::Markdown | FileType::Html => {
                            let mut dest = matcher::destination(source, target, path, &file_type, clean_url);
                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                        },
                        _ => {},
                    }

                } else {
                    // Ignore self
                    if path == parent.as_ref() {
                        continue;
                    }

                    let mut dir_index = path.to_path_buf();
                    dir_index.push("index");
                    let candidates = vec![
                        dir_index.with_extension("md"),
                        dir_index.with_extension("html")
                    ];

                    for f in candidates {
                        if f.exists() {
                            let file_type = matcher::get_type(layout, &f);
                            let mut dest = matcher::destination(source, target, &f, &file_type, clean_url);
                            if let Ok(cleaned) = dest.strip_prefix(target) {
                                dest = cleaned.to_path_buf();
                            }
                            if let Ok(rel) = dest.strip_prefix(rel_base) {
                                dest = rel.to_path_buf();
                            }
                            href = dest.to_string_lossy().to_string();
                        }
                    }
                }

                if !href.is_empty() {
                    if clean_url {
                        let index_name = "index.html";
                        if href.ends_with(index_name) {
                            href.truncate(href.len() - index_name.len());
                        }
                    }
                    let e = TocEntry{href: href};
                    entries.push(e);
                }
            }, Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, e));
            },
        }
    }

    Ok(entries)
}

// implement by a structure impls HelperDef
#[derive(Clone, Copy)]
pub struct Toc;

impl HelperDef for Toc {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

    let data = ctx.data();

    if let Some(file_context) = data.get("context") {
        if let Some(fp) = file_context.get("file") {
            if let Some(fp) = fp.as_str() {
                let path = Path::new(&fp);
                if let Some(parent) = path.parent() {
                    let entries = get_files(path, parent, file_context).unwrap();
                    let template = h.template();

                    match template {
                        Some(t) => {

                            for li in entries {
                                let mut context: BTreeMap<String, Value> = BTreeMap::new();

                                let href = &li.href;
                                context.insert("href".to_owned(), Value::String(href.to_owned()));

                                let mut local_rc = rc.clone();
                                let local_ctx = Context::wraps(&context)?;
                                t.render(r, &local_ctx, &mut local_rc, out)?;
                            }

                            return Ok(())
                        },
                        None => return Ok(())
                    }

                }
            }
        }
    }

    Ok(())
  }
}
