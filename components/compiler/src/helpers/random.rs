use handlebars::*;
use rand::seq::SliceRandom;
use serde_json::json;

use config::Page;

#[derive(Clone, Copy)]
pub struct Random;

impl HelperDef for Random {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let list = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new("Type error in `random`, expected parameter")
            })?
            .value()
            .as_array()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `random`, expected array parameter",
                )
            })?;

        let template = h.template().ok_or_else(|| {
            RenderError::new("Type error in `random`, block template expected")
        })?;

        let block_context = BlockContext::new();
        rc.push_block(block_context);

        if let Some(entry) = list.choose(&mut rand::thread_rng()) {
            if let Some(ref mut block) = rc.block_mut() {
                block.set_base_value(json!(entry));
            }
            template.render(r, ctx, rc, out)?;
        }

        rc.pop_block();

        Ok(())
    }
}
