use std::sync::Arc;

use handlebars::*;

use crate::{BuildContext, Result};
use collator::Collate;

pub fn get_permalink<'a>(
    href: Option<&'a str>,
    permalink: Option<&'a str>,
    context: &'a BuildContext,
) -> Result<String> {
    let base = context.options.get_canonical_url(
        &context.config,
        Some(context.collation.get_lang()),
    )?;

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

    get_permalink(href, permalink, context)
}

#[derive(Clone)]
pub struct PermaLink {
    pub context: Arc<BuildContext>,
}

impl HelperDef for PermaLink {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Ok(href) = get_permalink_href(ctx, &self.context) {
            out.write(&href)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Link {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Link {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Ok(href) = get_permalink_href(ctx, &self.context) {
            let markup = format!("<link rel=\"bookmark\" href=\"{}\">", &href);
            out.write(&markup)?;
        }
        Ok(())
    }
}
