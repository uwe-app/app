use handlebars::*;
use serde_json::{from_value, json};

use super::super::context::Context as BuildContext;

#[derive(Clone, Copy)]
pub struct LiveReload;

static STYLE: &str = "#livereload-notification {
    background: black;
    color: white;
    z-index: 999991;
    position: fixed;
    bottom: 0;
    left: 0;
    font-family: sans-serif;
    font-size: 14px;
    padding: 10px;
    border-top-right-radius: 6px;
    display: none;
}";

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
            let mut content = "".to_string();
            content.push_str(&format!("<style>{}</style>", STYLE));
            content.push_str("<div id='livereload-notification'><span>Building...</span></div>");
            content.push_str("<script src=\"/__livereload.js\"></script>");
            out.write(&content)?;
        }

        Ok(())
    }
}
