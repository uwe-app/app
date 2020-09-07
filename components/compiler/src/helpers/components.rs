use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;
use serde_json::Value;

use crate::tree;
use crate::BuildContext;

#[derive(Clone)]
pub struct Components {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Components {
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
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.source`, string expected",
                )
            })?
            .to_string();

        let template = h.template().ok_or_else(|| {
            RenderError::new(
                "Type error in `components`, block template expected",
            )
        })?;

        let source_path = PathBuf::from(&base_path);
        let components = tree::ancestors(&self.context, &source_path);
        let amount = components.len() - 1;

        for (i, page) in components.iter().rev().enumerate() {
            let first = i == 0;
            let last = i == amount;
            let href = std::iter::repeat("..")
                .take(amount - i)
                .collect::<Vec<_>>()
                .join("/");

            let mut local_rc = rc.clone();
            let mut local_ctx = Context::wraps(page)?;
            let ctx_data = local_ctx.data_mut().as_object_mut().unwrap();

            ctx_data.insert("href".to_string(), Value::String(href));
            ctx_data.insert("first".to_string(), Value::Bool(first));
            ctx_data.insert("last".to_string(), Value::Bool(last));

            template.render(r, &local_ctx, &mut local_rc, out)?;
        }

        Ok(())
    }
}
