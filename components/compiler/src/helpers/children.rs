use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;
use serde_json::Value;

use crate::tree::{self, ListOptions};
use crate::BuildContext;

#[derive(Clone)]
pub struct Children {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Children {
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
                "Type error in `children`, block template expected",
            )
        })?;

        let path = PathBuf::from(&base_path);
        let dir = path.parent().unwrap().to_path_buf();

        // TODO: See if we should render a specific directory

        let list_opts = ListOptions {
            sort: Some("title".to_string()),
            dir: &dir,
            depth: 1,
        };

        let list_result = tree::listing(&self.context, &list_opts);
        match list_result {
            Ok(entries) => {
                for li in entries {
                    let li = &*li.read().unwrap();

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
                return Ok(());
            }
            // FIXME: find a better way to convert these errors
            // SEE: https://stackoverflow.com/a/58337971/7625589
            Err(e) => return Err(RenderError::new(e.to_string())),
        }
    }
}
