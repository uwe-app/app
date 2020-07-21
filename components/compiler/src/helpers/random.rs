use handlebars::*;
use rand::seq::SliceRandom;
use serde_json::json;

use config::Page;

use super::with_parent_context;

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

        let list = h.params().get(0)
            .ok_or_else(|| RenderError::new("Type error in `random`, expected parameter"))?
            .value()
            .as_array()
            .ok_or_else(|| RenderError::new("Type error in `random`, expected array parameter"))?;

        let template = h.template()
            .ok_or_else(|| RenderError::new("Type error in `random`, block template expected"))?;

        if let Some(element) = list.choose(&mut rand::thread_rng()) {
            let mut local_rc = rc.clone();
            let mut data: Page = Default::default();

            data.extra.insert("entry".to_string(), json!(element));

            let local_ctx = with_parent_context(ctx, &mut data)?;
            template.render(r, &local_ctx, &mut local_rc, out)?;
            return Ok(());
        }

        Ok(())
    }
}
