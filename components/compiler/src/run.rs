use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::info;

use collator::Collate;
use config::{CollatedPage, Config, Page, ProfileName};

use crate::context::BuildContext;
use crate::draft;
use crate::parser::Parser;
use crate::Result;

use config::transform::HtmlTransformFlags;
use transform::text::TextExtraction;

#[derive(Debug)]
pub struct ParseData {
    pub file: PathBuf,
    pub extract: Option<TextExtraction>,
}

impl ParseData {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            extract: None,
        }
    }
}

fn is_html_extension<P: AsRef<Path>>(dest: P) -> bool {
    if let Some(ext) = dest.as_ref().extension() {
        return ext == config::HTML;
    }
    false
}

fn should_minify_html<P: AsRef<Path>>(
    dest: P,
    tag: &ProfileName,
    release: bool,
    config: &Config,
) -> bool {
    let html_extension = is_html_extension(dest.as_ref());
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

pub async fn copy<'a>(
    file: &PathBuf,
    dest: &PathBuf,
) -> Result<()> {
    info!("{} -> {}", file.display(), dest.display());
    utils::fs::copy(file, dest)?;
    Ok(())
}

pub async fn link<'a>(
    file: &PathBuf,
    dest: &PathBuf,
) -> Result<()> {
    info!("{} -> {}", file.display(), dest.display());

    // NOTE: prevent errors trying to symlink when the target
    // NOTE: already exists, otherwise when live reload is enabled
    // NOTE: the compiler errors will cause the websocket build
    // NOTE: complete message to never fire and the browser client
    // NOTE: will hang whilst building :(
    if dest.exists() {
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let abs = file.canonicalize()?;
    utils::symlink::soft(&abs, dest)?;
    Ok(())
}

pub async fn parse(
    ctx: Arc<BuildContext>,
    parser: &Parser<'_>,
    file: &PathBuf,
    data: &Page,
    dest: &PathBuf,
) -> Result<Option<ParseData>> {
    if draft::is_draft(&data, &ctx.options) {
        return Ok(None);
    }

    info!("{} -> {}", file.display(), dest.display());

    let minify_html = should_minify_html(
        &dest,
        &ctx.options.settings.name,
        ctx.options.settings.is_release(),
        &ctx.config,
    );

    let standalone = data.standalone.is_some();
    let lang = ctx.collation.get_lang();
    let page_data = CollatedPage { page: data, lang };

    let mut s = if minify_html {
        minify::html(parser.parse(file, page_data, standalone)?)
    } else {
        parser.parse(file, page_data, standalone)?
    };

    let mut res = ParseData::new(data.file.as_ref().unwrap().source.clone());

    if is_html_extension(&dest) {
        // Should we use text extraction?
        let mut use_text = ctx.config.search.is_some();
        // Set up the default transform flags
        let mut html_flags: HtmlTransformFlags = Default::default();

        // Do we need to perform any transformations?
        let mut requires_transform = ctx.config.search.is_some();

        if let Some(ref transform) = ctx.config.transform {
            if let Some(ref html) = transform.html {
                // Must use the config flags
                html_flags = html.clone();

                // Enable transform actions when necessary
                if !use_text {
                    use_text = html.use_words();
                }
                if !requires_transform {
                    requires_transform = html.is_active()
                }
            }
        }

        if requires_transform {
            let mut cache = transform::cache::TransformCache::new()?;
            cache.syntax_highlight =
                Some(ctx.config.is_syntax_enabled(&ctx.options.settings.name));

            cache.text = if use_text {
                Some(transform::text::TextExtraction::new())
            } else {
                None
            };

            if html_flags.is_active() || cache.is_active() {
                s = transform::html::apply(&s, &html_flags, &mut cache)?;
                // Assign the extracted text so we can use it later
                // to build the search index
                res.extract = cache.text.clone();
            }
        }
    }

    utils::fs::write_string(&dest, &s)?;

    Ok(Some(res))
}
