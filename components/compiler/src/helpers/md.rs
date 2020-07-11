use std::path::PathBuf;
use std::borrow::Cow;

use handlebars::*;

use serde_json::{json, from_value};

use super::super::markdown::render_markdown_string;
use super::super::context::Context as BuildContext;

use super::BufferedOutput;

#[derive(Clone, Copy)]
pub struct Markdown;

impl HelperDef for Markdown {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        // This dance let's us accept unquoted paths as a parameter
        // so we can accept {{md partial}} and {{md "partial"}} which makes
        // the behavior more like the normal include syntax {{> partial}}
        let template_name: String = if let Some(path_json) = h.param(0) {
            if path_json.is_value_missing() {
                if let Some(ref path) = path_json.relative_path() {
                    path.to_string()
                } else {
                    "".to_string()
                }
            } else {
                path_json
                    .value()
                    .as_str()
                    .ok_or(RenderError::new(
                        "Type error for `md` helper, first parameter must be a string"
                    ))?
                    .to_string()
            }
        } else {
            "".to_string()
        };

        let mut evaluate = h.param(1)
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `md` helper, second parameter must be a boolean"
            ))?;

        let source_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
            .replace("\"", "");

        let cfg = rc
            .evaluate(ctx, "@root/context")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `context`, map expected"))?
            .to_owned();

        let build_ctx: BuildContext = from_value(json!(cfg)).unwrap();
        let extensions = build_ctx.config.extension.as_ref().unwrap();
        let template = r.get_template(&template_name)
            .ok_or(
                RenderError::new(
                    format!("Type error for `md` helper, no template found for {}", &template_name)
                )
            )?;

        if !evaluate {
            let source_buf = PathBuf::from(&source_path);
            if let Some(ext) = source_buf.extension() {
                let s = ext.to_string_lossy().into_owned();
                evaluate = !extensions.markdown.contains(&s);
            }
        }

        let mut buf = BufferedOutput {
            buffer: "".to_owned(),
        };

        template.render(r, ctx, rc, &mut buf)?;

        //println!("md: {:?}", template_name);
        //println!("md: {:?}", evaluate);
        //println!("md: {:?}", &buf.buffer);

        if evaluate {
            let parsed = render_markdown_string(&mut Cow::from(buf.buffer), &build_ctx.config);
            out.write(&parsed)?;
        } else {
            out.write(&buf.buffer)?;
        }

        Ok(())
    }
}
