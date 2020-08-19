use std::path::Path;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;
use config::script::ScriptFile;

#[derive(Clone, Copy)]
pub struct Scripts<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Scripts<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        // Make links absolute (passthrough)
        let abs = h
            .hash_get("abs")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `scripts` helper, hash parameter `abs` must be a boolean",
            ))?;

        // List of page specific scripts
        let scripts = ctx
            .data()
            .as_object()
            .ok_or_else(
                || RenderError::new("Type error for `scripts` helper, invalid page data"))
            .unwrap()
            .get("scripts")
            .and_then(|v| v.as_array());

        // Get page-level scripts
        let mut scripts = if let Some(scripts) = scripts {
            scripts
                .iter()
                .map(|v| serde_json::from_value::<ScriptFile>(v.clone()).unwrap())
                .collect()
        } else {
            vec![]
        };

        // Append global scripts from the settings
        if let Some(ref js) = self.context.config.scripts {
            let mut main = js.main.clone();
            scripts.append(&mut main);
        }

        // Convert to relative paths if necessary
        let scripts = if abs {
            scripts
        } else {
            let opts = &self.context.options;
            let base_path = rc
                .evaluate(ctx, "@root/file.source")?
                .as_json()
                .as_str()
                .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
                .to_string();
            let path = Path::new(&base_path);

            scripts
                .iter()
                .map(|script| {
                    let mut tag = script.to_tag();
                    tag.src = config::link::relative(script.get_source(), path, &opts.source, opts)
                        .map_err(|_e| RenderError::new("Type error for `scripts`, file is outside source!"))
                        .unwrap();
                    ScriptFile::Tag(tag)
                })
                .collect()
        };

        for script in scripts {
            out.write(&script.to_string())?;
        }

        // Render block inline scripts
        if let Some(tpl) = h.template() {
            out.write("<script>")?;
            tpl.render(r, ctx, rc, out)?;
            out.write("</script>")?;
        }

        // Render `noscript` on the inverse
        if let Some(tpl) = h.inverse() {
            out.write("<noscript>")?;
            tpl.render(r, ctx, rc, out)?;
            out.write("</noscript>")?;
        }

        Ok(())
    }
}
