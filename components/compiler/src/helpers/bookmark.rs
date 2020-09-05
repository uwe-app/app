use handlebars::*;

use crate::{BuildContext, Result};
use config::{Config, RuntimeOptions};

pub fn get_permalink<'a>(
    href: Option<&'a str>,
    permalink: Option<&'a str>,
    config: &Config,
    opts: &RuntimeOptions,
) -> Result<String> {
    let base = opts.get_canonical_url(config, true)?;

    let path = if let Some(ref href) = permalink {
        href
    } else {
        href.as_ref().unwrap()
    };

    Ok(base.join(path)?.to_string())
}

fn get_permalink_href<'rc, 'a>(
    ctx: &'rc Context,
    context: &'a BuildContext,
) -> Result<String> {
    let href = ctx
        .data()
        .as_object()
        .ok_or_else(|| {
            RenderError::new("Type error for `bookmark`, invalid page data")
        })
        .unwrap()
        .get("href")
        .ok_or_else(|| {
            RenderError::new("Type error for `bookmark`, no href set")
        })
        .unwrap()
        .as_str();

    let permalink = ctx
        .data()
        .as_object()
        .ok_or_else(|| {
            RenderError::new("Type error for `bookmark`, invalid page data")
        })
        .unwrap()
        .get("permalink")
        .and_then(|v| v.as_str());

    get_permalink(href, permalink, &context.config, &context.options)
}

#[derive(Clone, Copy)]
pub struct PermaLink<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for PermaLink<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Ok(href) = get_permalink_href(ctx, self.context) {
            out.write(&href)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct Link<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Link<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Ok(href) = get_permalink_href(ctx, self.context) {
            let markup = format!("<link rel=\"bookmark\" href=\"{}\">", &href);
            out.write(&markup)?;
        }
        Ok(())
    }
}
