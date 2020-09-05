use std::path::PathBuf;

use crate::{BuildContext, Result};
use collator::Collate;
use config::Page;

#[derive(Debug)]
pub struct ListOptions<'a> {
    pub sort: Option<String>,
    pub dir: &'a PathBuf,
    pub depth: usize,
}

pub fn parent<'a>(ctx: &'a BuildContext, file: &PathBuf) -> Option<&'a Page> {
    let types = ctx.options.settings.types.as_ref().unwrap();
    let render_types = types.render();

    let skip = if let Some(stem) = file.file_stem() {
        if stem == config::INDEX_STEM {
            1
        } else {
            0
        }
    } else {
        0
    };

    for p in file.ancestors().skip(skip + 1).take(1) {
        let mut parent = p.join(config::INDEX_STEM);
        for ext in render_types.iter() {
            parent.set_extension(ext);
            if let Some(ref page) = ctx.collation.resolve(&parent) {
                return Some(page);
            }
        }
    }

    None
}

pub fn ancestors<'a>(ctx: &'a BuildContext, file: &PathBuf) -> Vec<&'a Page> {
    let mut pages: Vec<&'a Page> = Vec::new();
    let types = ctx.options.settings.types.as_ref().unwrap();
    let render_types = types.render();

    let skip = if let Some(stem) = file.file_stem() {
        if stem == config::INDEX_STEM {
            1
        } else {
            0
        }
    } else {
        0
    };

    for p in file.ancestors().skip(skip) {
        if let Some(ref page) = ctx.collation.resolve(&p.to_path_buf()) {
            pages.push(page);
            continue;
        }

        let mut parent = p.join(config::INDEX_STEM);
        for ext in render_types.iter() {
            parent.set_extension(ext);
            if let Some(ref page) = ctx.collation.resolve(&parent) {
                pages.push(page);
            }
        }
        if p == ctx.options.source {
            break;
        }
    }

    pages
}

pub fn listing<'a>(
    ctx: &'a BuildContext,
    list: &'a ListOptions,
) -> Result<Vec<&'a Page>> {
    let depth = list.dir.components().count() + list.depth;

    //let pages = ctx
    //.collation
    //.pages
    ////.get(&ctx.options.locales.fallback)
    //.unwrap();

    let keys = ctx
        .collation
        .pages()
        .filter(|(k, _)| {
            let key_count = k.components().count();
            if key_count == depth + 1 {
                if let Some(stem) = k.file_stem() {
                    stem == config::INDEX_STEM
                } else {
                    false
                }
            } else {
                k.starts_with(&list.dir) && key_count <= depth
            }
        })
        .map(|(k, _)| k)
        .collect::<Vec<_>>();

    let mut values = keys
        .iter()
        .map(|k| ctx.collation.resolve(*k).unwrap())
        .collect::<Vec<_>>();

    if let Some(ref sort_key) = list.sort {
        values.sort_by(|a, b| {
            let mut s1 = "";
            let mut s2 = "";
            if sort_key == "title" {
                s1 = a.title.as_ref().map(|x| &**x).unwrap_or("");
                s2 = b.title.as_ref().map(|x| &**x).unwrap_or("");
            }
            s1.partial_cmp(s2).unwrap()
        });
    }

    Ok(values)
}
