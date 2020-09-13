use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;

use collator::Collate;
use config::MenuType;

use crate::markdown::render_markdown;
use crate::BuildContext;

#[derive(Clone)]
pub struct Menu {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Menu {
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
                RenderError::new(
                    "Type error for `file.source`, string expected",
                )
            })?
            .to_string();

        let key = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `menu`, expected parameter at index 0",
                )
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `menu`, expected string parameter at index 0",
                )
            })?;

        let source_file = PathBuf::from(&source_path);
        let collation = self.context.collation.read().unwrap();
        if let Some(ref menu_result) = collation.find_menu(&source_file, key) {
            // TODO: use render_with_context()
            let mut result = r
                .render_template_with_context(&menu_result.value, ctx)
                .map_err(|e| {
                    RenderError::new(format!(
                        "Menu error {} ({})",
                        &source_path, e
                    ))
                })?;

            match menu_result.kind {
                // When we are in the context of an HTML page and
                // we encounter a menu template formatted as markdown
                // it needs to be transformed to HTML before being written
                MenuType::Markdown => {
                    result = render_markdown(
                        &mut Cow::from(result),
                        &self.context.config,
                    );
                }
                _ => {}
            }

            out.write(&result)?;
        }

        Ok(())
    }
}
