use std::borrow::Cow;
use std::path::PathBuf;

use handlebars::*;

use serde_json::json;

use super::BufferedOutput;
use crate::markdown::render_markdown_string;
use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct Markdown<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Markdown<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let source_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new("Type error in `md` for `file.source`, string expected")
            })?
            .to_string();

        let types = self.context.options.settings.types.as_ref().unwrap();

        let mut buf = BufferedOutput {
            buffer: "".to_owned(),
        };

        let mut evaluate = h
            .hash_get("render")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `md` helper, hash parameter `render` must be a boolean",
            ))?;

        // Parsing from block element
        if let Some(block) = h.template() {
            block.render(r, ctx, rc, &mut buf)?;
        // Parse from parameters
        } else {
            if let Some(path_json) = h.param(0) {
                // Handle path style partial template lookup {md partial}
                if path_json.is_value_missing() {
                    if let Some(ref path) = path_json.relative_path() {
                        let template = r.get_template(path).ok_or(RenderError::new(format!(
                            "Type error for `md` helper, no template found for {}",
                            path
                        )))?;
                        template.render(r, ctx, rc, &mut buf)?;
                    } else {
                        return Err(RenderError::new(
                            "Type error for `md` helper, unable to determine relative path",
                        ));
                    }
                } else {
                    let param = h
                        .param(0)
                        .map(|v| v.value())
                        .ok_or(RenderError::new(
                            "Type error for `md` helper, failed to get parameter",
                        ))?
                        .as_str()
                        .ok_or(RenderError::new(
                            "Type error for `md` helper, parameter should be a string",
                        ))?;

                    buf.buffer = param.to_string();

                    //println!("Got inline string buffer {:?}", &param);
                }
            }
        }

        //println!("md: {:?}", template_name);
        //println!("md: {:?}", evaluate);
        //println!("md: {:?}", &buf.buffer);

        if !evaluate {
            let source_buf = PathBuf::from(&source_path);
            if let Some(ext) = source_buf.extension() {
                let s = ext.to_string_lossy().into_owned();
                evaluate = !types.markdown().contains(&s);
            }
        }

        if evaluate {
            let parsed = render_markdown_string(&mut Cow::from(buf.buffer), &self.context.config);
            out.write(&parsed)?;
        } else {
            out.write(&buf.buffer)?;
        }

        Ok(())
    }
}
