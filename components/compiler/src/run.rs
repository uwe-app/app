use std::path::{Path, PathBuf};

use log::info;

use serde_json::{json, Value};

use config::{Config, Page, FileInfo, FileOptions, ProfileName, IndexQuery};

use crate::{Error, Result, HTML};
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

fn data_source_each(
    ctx: &BuildContext,
    parser: &Parser,
    file: &PathBuf,
    data: &Page,
    _reference: IndexQuery,
    values: Vec<Value>,
) -> Result<()> {

    let parent = file.parent().unwrap();

    let mut rewrite_index = ctx.options.settings.should_rewrite_index();
    // Override with rewrite-index page level setting
    if let Some(val) = data.rewrite_index {
        rewrite_index = val;
    }

    // Write out the document files
    for doc in &values {
        let mut item_data = data.clone();

        if let Some(id) = doc.get("id") {

            if let Some(id) = id.as_str() {
                if doc.is_object() {
                    let map = doc.as_object().unwrap();
                    for (k, v) in map {
                        item_data.extra.insert(k.clone(), json!(v));
                    }
                } else {
                    return Err(Error::DataSourceDocumentNotAnObject);
                }

                // Mock a source file to build a destination
                // respecting the clean URL setting
                let mut mock = parent.to_path_buf();
                mock.push(&id);
                if let Some(ext) = file.extension() {
                    mock.set_extension(ext);
                }

                let mut file_info = FileInfo::new(
                    &ctx.config,
                    &ctx.options,
                    &mock,
                    true,
                );

                let file_opts = FileOptions {
                    rewrite_index,
                    base_href: &ctx.options.settings.base_href,
                    ..Default::default()
                };

                file_info.destination(&file_opts)?;
                let dest = file_info.output.clone().unwrap();

                // Must inherit the real input template file
                file_info.file = file;

                item_data.seal(
                    &ctx.config,
                    &ctx.options,
                    &file_info)?;

                info!("{} -> {}", &id, &dest.display());

                let minify_html = should_minify_html(
                    &dest,
                    &ctx.options.settings.name,
                    ctx.options.settings.is_release(),
                    &ctx.config);

                let s = if minify_html {
                    minify::html(parser.parse(&file, &mut item_data)?)
                } else {
                    parser.parse(&file, &mut item_data)?
                };

                utils::fs::write_string(&dest, &s)?;
            }
        } else {
            return Err(Error::DataSourceDocumentNoId);
        }
    }

    Ok(())
}

fn parse_query(ctx: &BuildContext, parser: &Parser, file: &PathBuf, data: &mut Page) -> Result<bool> {
    if let Some(ref q) = data.query {
        let queries = q.clone().to_vec();
        let datasource = &ctx.datasource;

        if !datasource.map.is_empty() {
            let mut each_iters: Vec<(IndexQuery, Vec<Value>)> = Vec::new();
            for query in queries {
                let each = query.each.is_some() && query.each.unwrap();
                let idx = datasource.query_index(&query)?;

                // Push on to the list of generators to iterate
                // over so that we can support the same template
                // for multiple generator indices although not sure
                // how useful/desirable it is to declare multiple each iterators
                // as identifiers may well collide.
                if each {
                    each_iters.push((query, idx));
                } else {
                    data.extra.insert(query.get_parameter(), json!(idx));
                }
            }

            if !each_iters.is_empty() {
                for (gen, idx) in each_iters {
                    data_source_each(ctx, parser, file, &data, gen, idx)?;
                }
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub async fn copy(ctx: &BuildContext, file: &PathBuf) -> Result<()> {

    let mut info = FileInfo::new(
        &ctx.config,
        &ctx.options,
        file,
        false,
    );

    let file_opts = FileOptions {
        exact: true,
        base_href: &ctx.options.settings.base_href,
        ..Default::default()
    };

    info.destination(&file_opts)?;

    let dest = info.output.as_ref().unwrap();

    //let file = info.file;

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
