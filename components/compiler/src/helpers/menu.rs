use std::sync::Arc;

use handlebars::*;

use crate::BuildContext;

#[derive(Clone)]
pub struct Menu {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Menu {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let source_path = rc
            .evaluate(ctx, "@root/file.template")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.template`, string expected",
                )
            })?
            .to_string();

        let key = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `menu`, expected parameter at index 0",
                )
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `menu`, expected string parameter at index 0",
                )
            })?;

        let menu = rc.evaluate(ctx, "@root/menu")?;
        let menu = menu
            .as_json()
            .as_object()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `menu`, object expected",
                )
            })?
            .get(key);

        if let Some(ref value) = menu {
            let content = value.as_object().unwrap()
                .get("result").unwrap()
                .as_str().unwrap();

            let result = r.render_template(&content, ctx.data()).map_err(|e| {
                RenderError::new(format!("Menu error {} ({})", &source_path, e))
            })?;

            out.write(&result)?;
        } else {
            return Err(
                RenderError::new(
                    format!("Failed to find menu for key `{}`", key)))
        }

        Ok(())
    }
}
