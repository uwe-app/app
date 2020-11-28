use std::path::PathBuf;
use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;
use collator::{Collate, LinkCollate};
use serde_json::json;

pub struct Page {
    pub context: Arc<BuildContext>,
}

impl Helper for Page {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..1)?;

        let node = ctx.assert_block(template)?;

        // The href or file system path
        let href_or_path = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let collation = self.context.collation.read().unwrap();
        let normalized_href = collation.normalize(&href_or_path);
        let page_path =
            if let Some(page_path) = collation.get_link(&normalized_href) {
                page_path.to_path_buf()
            } else {
                PathBuf::from(&href_or_path)
            };

        if let Some(page_lock) = collation.resolve(&page_path) {
            rc.push_scope(Scope::new());
            let page = page_lock.read().unwrap();
            if let Some(ref mut block) = rc.scope_mut() {
                block.set_base_value(json!(&*page));
            }
            rc.template(node)?;
            rc.pop_scope();
        } else {
            return Err(HelperError::new(&format!(
                "Type error in `{}`, no page found for {}",
                ctx.name(),
                &href_or_path
            )));
        }

        Ok(None)
    }
}
