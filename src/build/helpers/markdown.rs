use handlebars::*;
use super::render_buffer;

use crate::markdown::render_markdown_string;

#[derive(Clone, Copy)]
pub struct Markdown;

impl HelperDef for Markdown {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let result = render_buffer(h, r, ctx, rc);
        match result {
            Ok(ref md) => {
                let result = render_markdown_string(md);
                out.write(&result)?;
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }
}
