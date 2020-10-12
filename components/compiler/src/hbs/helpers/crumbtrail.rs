use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use collator::menu;

use crate::BuildContext;

#[derive(Clone)]
pub struct Components {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Components {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
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

        let template = h.template().ok_or_else(|| {
            RenderError::new(
                "Type error in `components`, block template expected",
            )
        })?;

        let source_path = PathBuf::from(&base_path);

        let collation = self.context.collation.read().unwrap();
        let components =
            menu::components(&self.context.options, &*collation, &source_path);
        let amount = components.len() - 1;

        let block_context = BlockContext::new();
        rc.push_block(block_context);

        for (i, page) in components.iter().rev().enumerate() {
            let page = &*page.read().unwrap();
            let first = i == 0;
            let last = i == amount;
            let href = std::iter::repeat("..")
                .take(amount - i)
                .collect::<Vec<_>>()
                .join("/");

            if let Some(ref mut block) = rc.block_mut() {
                block.set_local_var("@first".to_string(), json!(first));
                block.set_local_var("@last".to_string(), json!(last));
                block.set_local_var("@href".to_string(), json!(href));
                block.set_base_value(json!(page));
            }
            template.render(r, ctx, rc, out)?;
        }

        rc.pop_block();

        Ok(())
    }
}
