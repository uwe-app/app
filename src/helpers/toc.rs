use std::convert::AsRef;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use handlebars::*;
use ignore::WalkBuilder;
use serde_json::{json, Value, Map};

use crate::{
    matcher,
    DataLoader,
    FileType,
    Options,
    HTML,
    INDEX_HTML,
    INDEX_STEM,
    MD
};

type TocEntry = Map<String, Value>;

fn get_files<P: AsRef<Path>>(file: P, parent: P, opts: &Options) -> io::Result<Vec<TocEntry>> {
    let mut entries: Vec<TocEntry> = Vec::new();

    let source = &opts.source;
    let target = &opts.target;

    let rel_base = parent
        .as_ref()
        .strip_prefix(source)
        .unwrap_or(Path::new(""));

    let loader = DataLoader::new(opts);

    for result in WalkBuilder::new(parent.as_ref()).max_depth(Some(1)).build() {
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
                            loader.load(&path, &mut data);
                        }
                        _ => {}
                    }
                } else {
                    // Ignore self
                    if path == parent.as_ref() {
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
                            loader.load(&f, &mut data);
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
                return Err(io::Error::new(io::ErrorKind::Other, e));
            }
        }
    }

    Ok(entries)
}

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
        let base_path = rc
            .evaluate(ctx, "@root/context.file")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file`, string expected"))?
            .replace("\"", "");

        let opts = rc
            .evaluate(ctx, "@root/context.options")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `options`, map expected"))?
            .to_owned();

        let o: Options = serde_json::from_value(json!(opts)).unwrap();

        let mut path = Path::new(&base_path).to_path_buf();

        // See if we should render a specific directory
        // relative to the <input> source folder
        let mut dir = "".to_string();
        if let Some(d) = h.params().get(0) {
            let v = d.value();
            if let Some(val) = v.as_str() {
                dir = val.to_owned();
            }
        }

        // Resolve using a dir string argument
        if !dir.is_empty() {
            // Note that PathBuf.push() with a value of "/"
            // will make the entire path point to "/" and not
            // concatenate the path as expected so we use a
            // string instead
            let mut dir_target = o.source.to_string_lossy().to_string();
            dir_target.push_str(&dir);

            let dir_dest = Path::new(&dir_target);
            if !dir_dest.exists() || !dir_dest.is_dir() {
                return Err(RenderError::new("Path parameter for toc does not resolve to a directory"));
            }

            // Later we find the parent so this makes it consistent
            // with using a file as the input path
            dir_target.push_str(INDEX_HTML);
            path = PathBuf::from(dir_target);
        }

        //println!("toc {:?}", &o.source);
        //println!("toc {:?}", &path);

        if let Some(parent) = path.parent() {
            let entries = get_files(&path, &parent.to_path_buf(), &o).unwrap();
            let template = h.template();
            match template {
                Some(t) => {
                    for li in entries {
                        let mut local_rc = rc.clone();
                        let local_ctx = Context::wraps(&li)?;
                        t.render(r, &local_ctx, &mut local_rc, out)?;
                    }
                    return Ok(());
                }
                None => return Ok(()),
            }
        }


        Ok(())
    }
}
