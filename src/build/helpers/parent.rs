use std::path::Path;

use handlebars::*;
use serde_json::json;

use crate::build::loader;
use crate::build::matcher;

use crate::build::context::Context as BuildContext;

use super::map_render_error;
use super::with_parent_context;

#[derive(Clone, Copy)]
pub struct Parent;

impl HelperDef for Parent {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let base_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file`, string expected"))?
            .replace("\"", "");

        let cfg = rc
            .evaluate(ctx, "@root/context")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `context`, map expected"))?
            .to_owned();

        let build_ctx: BuildContext = serde_json::from_value(json!(cfg)).unwrap();

        let path = Path::new(&base_path).to_path_buf();

        if let Some(parent) = matcher::resolve_parent_index(&path) {
            let template = h.template();
            match template {
                Some(t) => {
                    let mut data = loader::compute(&parent, &build_ctx.config, true)
                        .map_err(map_render_error)?;
                    let mut local_rc = rc.clone();
                    let local_ctx = with_parent_context(ctx, &mut data)?;
                    t.render(r, &local_ctx, &mut local_rc, out)?;
                    return Ok(());
                }
                None => return Err(RenderError::new("Template expected for parent helper")),
            }
        }
        Ok(())
    }
}
