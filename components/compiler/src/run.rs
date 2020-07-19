use std::path::{Path, PathBuf};

use log::info;

use serde_json::{json};

use config::{Config, Page, ProfileName};

use crate::{Result, HTML};
use crate::context::BuildContext;
use crate::parser::Parser;
use crate::draft;

fn should_minify_html<P: AsRef<Path>>(dest: P, tag: &ProfileName, release: bool, config: &Config) -> bool {
    let mut html_extension = false;
    if let Some(ext) = dest.as_ref().extension() {
        html_extension = ext == HTML;
    }

    if html_extension {
        if let Some(ref minify) = config.minify {
            if let Some(ref html) = minify.html {
                if !html.profiles.is_empty() {
                    return html.profiles.contains(tag);
                }
            } 
        }
    }

    release && html_extension
}

fn parse_query(ctx: &BuildContext, parser: &Parser, file: &PathBuf, data: &mut Page) -> Result<bool> {
    if let Some(ref q) = data.query {
        let queries = q.clone().to_vec();
        let datasource = &ctx.datasource;
        if !datasource.map.is_empty() {
            //let mut each_iters: Vec<(IndexQuery, Vec<Value>)> = Vec::new();
            for query in queries {
                let each = query.each.is_some() && query.each.unwrap();
                let idx = datasource.query_index(&query)?;
                if !each {
                    data.extra.insert(query.get_parameter(), json!(idx));
                }
            }
        }
    }
    Ok(false)
}

pub async fn copy(file: &PathBuf, dest: &PathBuf) -> Result<()> {
    info!("{} -> {}", file.display(), dest.display());
    utils::fs::copy(file, &dest)?;
    Ok(())
}

pub async fn parse(ctx: &BuildContext, parser: &Parser<'_>, file: &PathBuf, data: &mut Page) -> Result<()> {
    if draft::is_draft(&data, &ctx.options) {
        return Ok(());
    }

    let quit = parse_query(ctx, parser, file, data)?;
    if quit {
        return Ok(())
    }

    let dest = data.file.as_ref().unwrap().target.clone();

    info!("{} -> {}", file.display(), dest.display());

    let minify_html = should_minify_html(
        &dest,
        &ctx.options.settings.name,
        ctx.options.settings.is_release(),
        &ctx.config);

    let s = if minify_html {
        minify::html(parser.parse(file, &data)?)
    } else {
        parser.parse(file, &data)?
    };

    utils::fs::write_string(&dest, &s)?;

    Ok(())
}
