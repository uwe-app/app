use handlebars::*;
use rand::seq::SliceRandom;

use serde_json::{json, Map};

use super::with_parent_context;

#[derive(Clone, Copy)]
pub struct Random;

impl HelperDef for Random{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let type_err = Err(
            RenderError::new("Type error for `random`, array parameter expected"));

        let template_err = Err(
            RenderError::new("Type error for `random`, inner template expected"));

        if let Some(p) = h.params().get(0) {
            if !p.is_value_missing() {
                let value = p.value(); 
                if value.is_array() {
                    let value = value.as_array().unwrap();
                    if let Some(element) = value.choose(&mut rand::thread_rng()) {
                        if let Some(t) = h.template() {
                            let mut local_rc = rc.clone();
                            let mut data = Map::new();
                            data.insert("entry".to_string(), json!(element));
                            let local_ctx = with_parent_context(ctx, &data)?;
                            t.render(r, &local_ctx, &mut local_rc, out)?;
                            return Ok(());
                        } else {
                            return template_err
                        }
                    }
                } else {
                    return type_err
                }
            } else {
                return type_err
            }
        } else {
            return type_err
        }

        Ok(())
    }
}

