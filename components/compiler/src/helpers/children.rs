use std::path::Path;

use handlebars::*;

use super::super::tree::{self, ListOptions};

#[derive(Clone, Copy)]
pub struct Children;

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
            .ok_or_else(|| RenderError::new("Type error for `file`, string expected"))?
            .replace("\"", "");

        let runtime = config::runtime::runtime().read().unwrap();
        let path = Path::new(&base_path).to_path_buf();

        // See if we should render a specific directory
        // relative to the <input> source folder
        let mut dir = "".to_string();
        if let Some(d) = h.params().get(0) {
            let v = d.value();
            if let Some(val) = v.as_str() {
                dir = val.to_owned();
            }
        }

        let list_opts = ListOptions {
            sort: true,
            sort_key: "title".to_string(),
            dir: dir.to_owned(),
            depth: 1,
        };

        let list_result = tree::listing(&path, &list_opts, &runtime.config, &runtime.options);
        match list_result {
            Ok(entries) => {
                let template = h.template();
                match template {
                    Some(t) => {
                        for li in entries {
                            let mut local_rc = rc.clone();
                            let local_ctx = Context::wraps(&li)?;
                            t.render(r, &local_ctx, &mut local_rc, out)?;
                        }
                        return Ok(());
                    }
                    None => return Ok(()),
                }
            }
            // FIXME: find a better way to convert these errors
            // SEE: https://stackoverflow.com/a/58337971/7625589
            Err(e) => return Err(RenderError::new(e.to_string())),
        }
    }
}
