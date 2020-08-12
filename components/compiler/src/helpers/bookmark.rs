use handlebars::*;
use log::warn;

use config::{Config, RuntimeOptions};
use crate::{Result, BuildContext};

pub fn get_permalink<'a>(
    href: Option<&'a str>,
    permalink: Option<&'a str>,
    config: &Config,
    opts: &RuntimeOptions) -> Result<String> {

    let base = opts.get_canonical_url(config, true)?;

    let path = if let Some(ref href) = permalink {
        href
    } else {
        href.as_ref().unwrap()
    };

    Ok(base.join(path)?.to_string())
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

        let href = ctx
            .data()
            .as_object()
            .ok_or_else(
                || RenderError::new("Type error for `bookmark`, invalid page data"))
            .unwrap()
            .get("href")
            .ok_or_else(
                || RenderError::new("Type error for `bookmark`, no href set"))
            .unwrap()
            .as_str();

        let permalink = ctx
            .data()
            .as_object()
            .ok_or_else(
                || RenderError::new("Type error for `bookmark`, invalid page data"))
            .unwrap()
            .get("permalink")
            .and_then(|v| v.as_str());

        if let Ok(href) = get_permalink(href, permalink, &self.context.config, &self.context.options) {
            let markup = format!("<link rel=\"bookmark\" href=\"{}\">", &href);
            out.write(&markup)?;
        } else {
            warn!("Failed to create bookmark for {}", href.unwrap());
        }
        Ok(())
    }
}
