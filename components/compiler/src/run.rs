use std::path::{Path, PathBuf};

use log::info;

use config::{Config, Page, ProfileName};

use crate::Result;
use crate::context::BuildContext;
use crate::parser::Parser;
use crate::draft;

use transform::text::TextExtraction;

#[derive(Debug)]
pub struct ParseData<'a> {
    pub file: &'a PathBuf,
    pub extract: Option<TextExtraction>,
}

impl<'a> ParseData<'a> {
    pub fn new(file: &'a PathBuf) -> Self {
        Self {
            file, 
            extract: None,
        } 
    }
}

fn should_minify_html<P: AsRef<Path>>(dest: P, tag: &ProfileName, release: bool, config: &Config) -> bool {
    let mut html_extension = false;
    if let Some(ext) = dest.as_ref().extension() {
        html_extension = ext == config::HTML;
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

pub async fn copy<'a>(file: &PathBuf, dest: &PathBuf) -> Result<Option<ParseData<'a>>> {
    info!("{} -> {}", file.display(), dest.display());
    utils::fs::copy(file, &dest)?;
    Ok(None)
}

pub async fn parse<'a>(ctx: &BuildContext, parser: &Parser<'_>, file: &'a PathBuf, data: &Page) -> Result<Option<ParseData<'a>>> {

    if draft::is_draft(&data, &ctx.options) {
        return Ok(None);
    }

    let dest = data.file.as_ref().unwrap().target.clone();

    info!("{} -> {}", file.display(), dest.display());

    let minify_html = should_minify_html(
        &dest,
        &ctx.options.settings.name,
        ctx.options.settings.is_release(),
        &ctx.config);

    let mut s = if minify_html {
        minify::html(parser.parse(file, &data)?)
    } else {
        parser.parse(file, &data)?
    };

    let mut res = ParseData::new(file);

    if let Some(ref transform) = ctx.config.transform {
        if let Some(ref html) = transform.html {

            let mut cache = transform::cache::TransformCache::new()?;
            cache.syntax_highlight =
                Some(
                    ctx.config.is_syntax_enabled(&ctx.options.settings.name));

            // TODO: also enable this for search indexing
            let use_text = html.use_words();

            cache.text = if use_text {
                Some(transform::text::TextExtraction::new())
            } else {
                None  
            };

            if html.is_active() || cache.is_active() {
                s = transform::html::apply(&s, &html, &mut cache)?;

                res.extract = cache.text.clone();

                //println!("{}", cache.text.as_ref().unwrap().to_string());
            }
        }
    }

    utils::fs::write_string(&dest, &s)?;

    Ok(Some(res))
}
