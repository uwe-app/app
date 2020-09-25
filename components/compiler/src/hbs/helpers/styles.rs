use std::path::Path;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;
use config::style::StyleAsset;

#[derive(Clone)]
pub struct Styles {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Styles {
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
                "Type error for `styles` helper, hash parameter `main` must be a boolean",
            ))?;

        // Make links absolute (passthrough)
        let abs = h
            .hash_get("abs")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `styles` helper, hash parameter `abs` must be a boolean",
            ))?;

        // List of page specific styles
        let styles = ctx
            .data()
            .as_object()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `styles` helper, invalid page data",
                )
            })
            .unwrap()
            .get("styles")
            .and_then(|v| v.as_array());

        // Get page-level styles
        let mut styles = if let Some(styles) = styles {
            styles
                .iter()
                .map(|v| {
                    serde_json::from_value::<StyleAsset>(v.clone()).unwrap()
                })
                .collect()
        } else {
            vec![]
        };

        // Use global styles from the settings
        if main {
            if let Some(ref css) = self.context.config.styles {
                let mut main = css.main.clone();
                styles.append(&mut main);
            }
        }

        // Convert to relative paths if necessary
        let styles = if abs {
            styles
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

            styles
                .iter()
                .map(|style| {
                    let mut tag = style.to_tag();
                    if let Some(ref src) = style.get_source() {
                        tag.href = Some(
                            opts.relative(src, path, &opts.source).unwrap(),
                        );
                    }
                    StyleAsset::Tag(tag)
                })
                .collect()
        };

        for style in styles {
            out.write(&style.to_string())?;
        }

        // Render block inline styles
        if let Some(tpl) = h.template() {
            out.write("<style>")?;
            tpl.render(r, ctx, rc, out)?;
            out.write("</style>")?;
        }

        Ok(())
    }
}
