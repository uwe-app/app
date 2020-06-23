use handlebars::*;
use serde_json::{from_value, json};

use crate::build::context::Context as BuildContext;

#[derive(Clone, Copy)]
pub struct LiveReload;

impl HelperDef for LiveReload {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let cfg = rc
            .evaluate(ctx, "@root/context")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `context`, map expected"))?
            .to_owned();

        let ctx: BuildContext = from_value(json!(cfg)).unwrap();
        if ctx.options.live {
            let script = "<script src=\"/__livereload.js\"></script>";
            out.write(script)?;
        }

        Ok(())
    }
}
