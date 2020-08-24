use handlebars::*;
use std::path::PathBuf;

use serde_json::Value;

use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct Series<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Series<'_> {
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
            .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
            .to_string();

        let path = PathBuf::from(&base_path);

        let name = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new("Type error in `series`, expected parameter at index 0")
            })?
            .value()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error in `series`, expected string parameter"))?;

        let template = h
            .template()
            .ok_or_else(|| RenderError::new("Type error in `series`, block template expected"))?;

        if let Some(set) = self.context.collation.series.get(name) {
            for p in set {
                if let Some(li) = self.context.collation.resolve(p, &self.context.options) {
                    let mut local_rc = rc.clone();
                    let mut local_ctx = Context::wraps(li)?;
                    if let Some(ref file_ctx) = li.file {
                        if file_ctx.source == path {
                            local_ctx
                                .data_mut()
                                .as_object_mut()
                                .unwrap()
                                .insert("self".to_string(), Value::Bool(true));
                        }
                    }
                    template.render(r, &local_ctx, &mut local_rc, out)?;
                }
            }
        } else {
            return Err(RenderError::new(format!(
                "Series `{}` does not exist",
                name
            )));
        }

        Ok(())
    }
}
