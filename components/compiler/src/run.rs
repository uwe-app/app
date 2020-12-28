use std::path::{Path, PathBuf};

use log::info;

use collator::{Resource, ResourceOperation, ResourceTarget};
use config::{
    profile::{ProfileName, Profiles},
    Config, Page,
};

use config::transform::HtmlTransformFlags;
use transform::text::TextExtraction;

use crate::{
    context::BuildContext, page::CollatedPage, parser::Parser, Error, Result,
};

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

/// Build a single file, negotiates pages and resource files.
pub async fn one(
    context: &BuildContext,
    parser: &Box<impl Parser + Send + Sync + ?Sized>,
    file: &PathBuf,
) -> Result<Option<ParseData>> {
    let collation = &*context.collation.read().unwrap();

    if let Some(target) = collation.get_resource(file) {
        match target.as_ref() {
            Resource::Page { ref target } => {
                if let Some(page) = collation.resolve(file) {
                    let page = &*page.read().unwrap();

                    match target.operation {
                        ResourceOperation::Render => {
                            //let rel = page.file.as_ref().unwrap().target.clone();
                            //let dest = context.collation.get_path().join(&rel);

                            let dest = target
                                .get_output(collation.get_path().as_ref());

                            return parse(
                                context,
                                parser,
                                page.get_template(),
                                page,
                                &dest,
                            )
                            .await;
                        }
                        _ => resource(context, file, target).await?,
                    }
                } else {
                    return Err(Error::PageResolve(file.to_path_buf()));
                }
            }
            Resource::File { ref target } => {
                resource(context, file, target).await?
            }
        }
    }

    /*
    match collation.get_resource(file).unwrap() {
        Resource::Page { ref target } => {
            if let Some(page) = collation.resolve(file) {
                let page = &*page.read().unwrap();

                match target.operation {
                    ResourceOperation::Render => {
                        //let rel = page.file.as_ref().unwrap().target.clone();
                        //let dest = context.collation.get_path().join(&rel);

                        let dest = target.get_output(collation.get_path());

                        return parse(
                            context,
                            parser,
                            page.get_template(),
                            page,
                            &dest,
                        )
                        .await;
                    }
                    _ => resource(context, file, target).await?,
                }
            } else {
                return Err(Error::PageResolve(file.to_path_buf()));
            }
        }
        Resource::File { ref target } => {
            resource(context, file, target).await?
        }
    }
    */

    Ok(None)
}

/// Handle a resource file depending upon the resource operation.
pub async fn resource(
    context: &BuildContext,
    file: &PathBuf,
    target: &ResourceTarget,
) -> Result<()> {
    let collation = &*context.collation.read().unwrap();
    match target.operation {
        ResourceOperation::Noop => Ok(()),
        ResourceOperation::Copy => {
            copy(file, &target.get_output(collation.get_path().as_ref())).await
        }
        ResourceOperation::Link => {
            link(file, &target.get_output(collation.get_path().as_ref())).await
        }
        _ => Err(Error::InvalidResourceOperation(file.to_path_buf())),
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
                return html.profiles().is_match(tag);
            }
        }
    }
    release && html_extension
}

async fn copy<'a>(file: &PathBuf, dest: &PathBuf) -> Result<()> {
    info!("{} -> {}", file.display(), dest.display());
    utils::fs::copy(file, dest)?;
    Ok(())
}

async fn link<'a>(file: &PathBuf, dest: &PathBuf) -> Result<()> {
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
    ctx: &BuildContext,
    parser: &Box<impl Parser + Send + Sync + ?Sized>,
    file: &PathBuf,
    data: &Page,
    dest: &PathBuf,
) -> Result<Option<ParseData>> {
    info!("{} -> {}", file.display(), dest.display());

    let minify_html = should_minify_html(
        &dest,
        &ctx.options.settings.name,
        ctx.options.settings.is_release(),
        &ctx.config,
    );

    let collation = &*ctx.collation.read().unwrap();
    let lang = collation.get_lang();
    let mut page_data = CollatedPage::new(
        file,
        &ctx.config,
        &ctx.options,
        &ctx.locales,
        data,
        lang.as_ref(),
    )?;

    page_data.menus = collation.menu_page_href();

    let mut s = if minify_html {
        minify::html(parser.parse(file, page_data)?)
    } else {
        parser.parse(file, page_data)?
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
