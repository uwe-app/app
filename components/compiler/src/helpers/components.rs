use std::path::Path;

use handlebars::*;
use serde_json::json;

use crate::lookup;
use crate::BuildContext;

use super::map_render_error;

#[derive(Clone, Copy)]
pub struct Components<'a> {
    pub context: &'a BuildContext
}

impl HelperDef for Components<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let base_path = rc
            .evaluate(ctx, "@root/file.target")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.target`, string expected"))?
            .replace("\"", "");

        let opts = &self.context.options;
        let path = Path::new(&base_path).to_path_buf();

        let template = h.template();
        match template {
            Some(t) => {
                let include_index = opts.settings.should_include_index();

                if let Ok(rel) = path.strip_prefix(&opts.target) {

                    let mut buf = rel.to_path_buf();
                    if buf.ends_with(config::INDEX_HTML) {
                        buf.pop();
                    }

                    let mut parts: Vec<String> = buf
                        .iter()
                        .map(|part| part.to_string_lossy().into_owned())
                        .collect();

                    // Add an empty string for home page
                    parts.insert(0, "/".to_string());

                    let up = "../".to_string();
                    let mut href = "".to_string();
                    for (pos, name) in parts.iter().enumerate() {
                        let amount = (parts.len() - 1) - pos;
                        let first = pos == 0;
                        let last = amount == 0;
                        if pos > 0 {
                            href.push('/');
                            href.push_str(&name);
                        }
                        let mut url = up.repeat(amount);
                        if include_index {
                            url.push_str(config::INDEX_HTML);
                        }

                        if let Some(src) = lookup::lookup(self.context, &href) {
                            let mut data = loader::compute(src, &self.context.config, &self.context.options, true)
                                .map_err(map_render_error)?;

                            data.extra.insert("first".to_string(), json!(first));
                            data.extra.insert("last".to_string(), json!(last));

                            data.href = Some(url);

                            let mut local_rc = rc.clone();
                            let local_ctx = Context::wraps(&data)?;
                            t.render(r, &local_ctx, &mut local_rc, out)?;
                        }
                    }
                }
            }
            None => return Err(RenderError::new("Template expected for components helper")),
        }

        Ok(())
    }
}
