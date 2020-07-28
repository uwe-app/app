use std::path::{Path, PathBuf};

use log::info;

use config::{Config, Page, ProfileName};

use crate::Result;
use crate::context::BuildContext;
use crate::parser::Parser;
use crate::draft;

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

pub async fn copy(file: &PathBuf, dest: &PathBuf) -> Result<()> {
    info!("{} -> {}", file.display(), dest.display());
    utils::fs::copy(file, &dest)?;
    Ok(())
}

pub async fn parse(ctx: &BuildContext, parser: &Parser<'_>, file: &PathBuf, data: &Page) -> Result<()> {
    if draft::is_draft(&data, &ctx.options) {
        return Ok(());
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

    let flags = transform::TransformFlags {
        strip_comments: true,
        auto_id: true,
        syntax_highlight: ctx.config.is_syntax_enabled(&ctx.options.settings.name),
    };

    //println!("Flags {:?}", flags);

    s = transform::apply(&s, &flags)?;

    utils::fs::write_string(&dest, &s)?;

    Ok(())
}
