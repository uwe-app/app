use std::path::Path;

use handlebars::*;
use serde_json::{json, Map};

use crate::{tree, BuildOptions};

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

        let opts = rc
            .evaluate(ctx, "@root/context.options")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `options`, map expected"))?
            .to_owned();

        let opts: BuildOptions = serde_json::from_value(json!(opts)).unwrap();
        let path = Path::new(&base_path).to_path_buf();

        //println!("got parent macro {:?}", path);

        let template = h.template();
        match template {
            Some(t) => {
                let mut data = Map::new();
                let result = tree::parent(&path, &opts, &mut data);
                match result {
                    Ok(_) => {
                        let mut local_rc = rc.clone();
                        let local_ctx = Context::wraps(&data)?;
                        t.render(r, &local_ctx, &mut local_rc, out)?;
                        return Ok(());
                    },
                    // FIXME: find a better way to convert these errors
                    // SEE: https://stackoverflow.com/a/58337971/7625589
                    Err(e) => return Err(RenderError::new(e.to_string()))
                }
            }
            None => return Err(RenderError::new("Template expected for parent helper")),
        }
    }
}

