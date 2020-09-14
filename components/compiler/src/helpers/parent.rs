use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use collator::menu;

use crate::BuildContext;

#[derive(Clone)]
pub struct Parent {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Parent {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let template = h.template().ok_or_else(|| {
            RenderError::new("Type error in `parent`, block template expected")
        })?;

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

        let path = PathBuf::from(&base_path);
        let collation = self.context.collation.read().unwrap();

        let block_context = BlockContext::new();
        rc.push_block(block_context);

        if let Some(page_lock) = menu::parent(&self.context.options, &*collation, &path) {
            let page = page_lock.read().unwrap();
            if let Some(ref mut block) = rc.block_mut() {
                block.set_base_value(json!(&*page));
            }
            template.render(r, ctx, rc, out)?;
        }

        rc.pop_block();

        Ok(())
    }
}
