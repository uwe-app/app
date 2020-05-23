use handlebars::*;

use crate::utils;
use super::render_buffer;

#[derive(Clone, Copy)]
pub struct Markdown;

impl HelperDef for Markdown{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Ok(ref md) = render_buffer(h, r, ctx, rc) {
            let result = utils::render_markdown_string(md);
            out.write(&result)?;
        }
        Ok(())
    }
}

