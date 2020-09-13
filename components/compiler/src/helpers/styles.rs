use std::path::Path;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;
use config::style::StyleFile;

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

        // Use global styles from the settings
        let mut sheets: Vec<StyleFile> =
            if let Some(ref styles) = self.context.config.styles {
                styles.main.clone()
            } else {
                vec![]
            };

        // NOTE: Unlike scripts which come beforehand page-level
        // NOTE: styles come afterwards following the principle of specificity
        if let Some(styles) = styles {
            let mut page_styles = styles
                .iter()
                .map(|v| {
                    serde_json::from_value::<StyleFile>(v.clone()).unwrap()
                })
                .collect();
            sheets.append(&mut page_styles);
        }

        // Convert to relative paths if necessary
        let sheets = if abs {
            sheets
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

            sheets
                .iter()
                .map(|style| {
                    let rel = opts
                        .relative(style.get_source(), path, &opts.source)
                        .map_err(|_e| {
                            RenderError::new(
                            "Type error for `styles`, file is outside source!",
                        )
                        })
                        .unwrap();
                    StyleFile::Source(rel)
                })
                .collect()
        };

        for style in sheets {
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
