use std::path::PathBuf;

use config::Page;
use crate::{Result, BuildContext};

#[derive(Debug)]
pub struct ListOptions {
    pub sort: Option<String>,
    pub dir: PathBuf,
    pub depth: usize,
}

pub fn listing<'a>(ctx: &'a BuildContext, list: &'a ListOptions) -> Result<Vec<&'a Page>> {
    let keys = ctx.collation.pages
        .iter()
        .filter(|(k, _)| {
            k.starts_with(&list.dir)
                && k.components().count() <= (list.dir.components().count() + list.depth)
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

