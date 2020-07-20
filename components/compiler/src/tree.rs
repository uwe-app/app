use std::path::PathBuf;

use config::Page;
use crate::{Result, BuildContext};

#[derive(Debug)]
pub struct ListOptions<'a> {
    pub sort: Option<String>,
    pub dir: &'a PathBuf,
    pub depth: usize,
}

pub fn ancestors<'a>(ctx: &'a BuildContext) -> Result<Vec<&'a Page>> {
    Ok(vec![])
}

pub fn listing<'a>(ctx: &'a BuildContext, list: &'a ListOptions) -> Result<Vec<&'a Page>> {

    let depth = list.dir.components().count() + list.depth;

    let keys = ctx.collation.pages
        .iter()
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
        .map(|k| ctx.collation.pages.get(*k).unwrap())
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

