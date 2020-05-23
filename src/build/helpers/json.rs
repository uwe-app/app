use handlebars::*;

use serde_json::to_string_pretty;

#[derive(Clone, Copy)]
pub struct Debug;

impl HelperDef for Debug{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Some(p) = h.params().get(0) {
            if let Ok(s) = to_string_pretty(p.value()) {
                out.write(&s)?;
            }
        } else {
            if let Ok(s) = to_string_pretty(ctx.data()) {
                out.write(&s)?;
            }
        }
        Ok(())
    }
}

