use std::path::Path;

use handlebars::*;
use serde_json::{json, Map, Value};

use crate::build::loader;
use crate::build::matcher;

#[derive(Clone, Copy)]
pub struct Parent;

impl HelperDef for Parent{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let base_path = rc
            .evaluate(ctx, "@root/context.file")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file`, string expected"))?
            .replace("\"", "");

        let path = Path::new(&base_path).to_path_buf();

        if let Some(parent) = matcher::resolve_parent_index(&path) {
            let template = h.template();
            match template {
                Some(t) => {
                    // This dance keeps the parent context data intact
                    // so that the `link` helper can be called inside this
                    // context
                    
                    let existing = ctx.data().as_object().unwrap();
                    let data = loader::compute(&parent);
                    let mut local_rc = rc.clone();

                    let mut new_data: Map<String, Value> = Map::new();
                    for (k, v) in existing {
                        new_data.insert(k.clone(), json!(v));
                    }
                    for (k, v) in &data {
                        new_data.insert(k.clone(), json!(v));
                    }

                    let local_ctx = Context::wraps(&new_data)?;
                    t.render(r, &local_ctx, &mut local_rc, out)?;
                    return Ok(());
                }
                None => return Err(RenderError::new("Template expected for parent helper")),
            }
        }
        Ok(())
    }
}

