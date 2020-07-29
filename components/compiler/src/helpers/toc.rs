use handlebars::*;

use serde_json::json;

#[derive(Clone, Copy)]
pub struct TableOfContents;

impl HelperDef for TableOfContents {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let from = h.hash_get("from")
            .map(|v| v.value())
            .or(Some(&json!("h1")))
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `toc` helper, hash parameter `from` must be a string"
            ))?.to_string();

        let to = h.hash_get("to")
            .map(|v| v.value())
            .or(Some(&json!("h6")))
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `toc` helper, hash parameter `to` must be a string"
            ))?.to_string();

        let el = format!("<toc data-from=\"{}\" data-to=\"{}\" />", &from, &to);
        out.write(&el)?;
        Ok(())
    }
}
