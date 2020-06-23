use std::path::Path;

use handlebars::*;

use crate::utils;
use log::debug;
//use super::render_buffer;

#[derive(Clone, Copy)]
pub struct Include;

impl HelperDef for Include {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
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

        // TODO: support embedding only certain lines only

        let mut buf = Path::new(&base_path).to_path_buf();

        if let Some(parent) = buf.parent() {
            buf = parent.to_path_buf();
            if let Some(req) = h.params().get(0) {
                // TODO: support using "value()" too?
                if let Some(val) = req.relative_path() {
                    buf.push(val);
                    debug!("include {}", buf.display());
                    let result = utils::read_string(&buf);
                    match result {
                        Ok(s) => {
                            out.write(&s)?;
                        }
                        Err(_) => {
                            return Err(RenderError::new(format!(
                                "Failed to read from include file: {}",
                                buf.display()
                            )))
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
