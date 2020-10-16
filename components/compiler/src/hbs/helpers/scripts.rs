use std::path::Path;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;
use config::script::ScriptAsset;

#[derive(Clone)]
pub struct Scripts {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Scripts {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Embed the main script
        let main = h
            .hash_get("main")
            .map(|v| v.value())
            .or(Some(&json!(true)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `scripts` helper, hash parameter `main` must be a boolean",
            ))?;

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
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `scripts` helper, invalid page data",
                )
            })
            .unwrap()
            .get("scripts")
            .and_then(|v| v.as_array());

        // Get page-level scripts
        let mut scripts = if let Some(scripts) = scripts {
            scripts
                .iter()
                .map(|v| {
                    serde_json::from_value::<ScriptAsset>(v.clone()).unwrap()
                })
                .collect()
        } else {
            vec![]
        };

        // Append global scripts from the settings
        if main {
            if let Some(ref js) = self.context.config.scripts {
                let mut main = js.main.clone();
                scripts.append(&mut main);
            }
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
                .ok_or_else(|| {
                    RenderError::new(
                        "Type error for `file.source`, string expected",
                    )
                })?
                .to_string();
            let path = Path::new(&base_path);

            scripts
                .iter()
                .map(|script| {
                    let mut tag = script.to_tag();
                    if let Some(ref src) = script.get_source() {
                        tag.src = Some(
                            opts.relative(src, path, &opts.source).unwrap(),
                        );
                    }
                    ScriptAsset::Tag(tag)
                })
                .collect()
        };

        for script in scripts {
            out.write(&script.to_string())?;
        }

        if self.context.options.settings.is_live() {
            let asset = ScriptAsset::Source(livereload::javascript());
            out.write(&asset.to_string())?;
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
