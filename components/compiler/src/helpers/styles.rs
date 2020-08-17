use std::path::Path;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct Styles<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Styles<'_> {
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

        // The main stylesheet (optional)
        let main = h
            .hash_get("main")
            .map(|v| v.value())
            .and_then(|v| v.as_str());

        // List of page specific styles
        let styles = ctx
            .data()
            .as_object()
            .ok_or_else(
                || RenderError::new("Type error for `styles` helper, invalid page data"))
            .unwrap()
            .get("styles")
            .and_then(|v| v.as_array());

        let mut sheets: Vec<&str> = if let Some(main_style) = main {
            vec![main_style] 
        } else {
            vec![]
        };

        if let Some(stylesheets) = styles {
            let mut page_sheets: Vec<&str> = stylesheets
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            sheets.append(&mut page_sheets);
        }

        let opts = &self.context.options;
        let base_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
            .to_string();
        let path = Path::new(&base_path);

        for href in sheets {
            let href = if abs {
                href.to_string()
            } else {
                config::link::relative(href, path, &opts.source, opts)
                    .map_err(|_e| RenderError::new("Type error for `styles`, file is outside source!"))?
            };

            let markup = format!("<link rel=\"stylesheet\" href=\"{}\">", href);
            out.write(&markup)?;
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
