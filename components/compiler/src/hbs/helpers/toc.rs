use bracket::helper::prelude::*;
use serde_json::json;

pub struct TableOfContents;

impl Helper for TableOfContents {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(0..0)?;

        let tag = ctx
            .param("tag")
            .or(Some(&json!("ol")))
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `toc` helper, hash parameter `tag` must be a string",
            ))?
            .to_string();

        let class = ctx
            .param("class")
            .or(Some(&json!("toc")))
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `toc` helper, hash parameter `class` must be a string",
            ))?
            .to_string();

        let from = ctx
            .param("from")
            .or(Some(&json!("h1")))
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `toc` helper, hash parameter `from` must be a string",
            ))?
            .to_string();

        let to = ctx
            .param("to")
            .or(Some(&json!("h6")))
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `toc` helper, hash parameter `to` must be a string",
            ))?
            .to_string();

        if tag != "ol" && tag != "ul" {
            return Err(HelperError::new(
                "Type error for `toc` helper, the tag name must be either `ol` or `ul`",
            ));
        }

        let el = format!(
            "<toc data-tag=\"{}\" data-class=\"{}\" data-from=\"{}\" data-to=\"{}\" />",
            &tag, &class, &from, &to
        );
        rc.write(&el)?;
        Ok(None)
    }
}
