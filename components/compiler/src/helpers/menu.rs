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


        /*
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
        let source_file = PathBuf::from(&source_path);
        */

        // TODO: handle file-specific menu overrides

        let collation = self.context.collation.read().unwrap();
        let menus = collation.get_graph().get_menus();
        let name = menus.get_menu_template_name(key);

        if let Some(_tpl) = r.get_template(&name) {
            let result = r.render_with_context(&name, ctx)?;
            out.write(&result)?;
        }

        Ok(())
    }
}
