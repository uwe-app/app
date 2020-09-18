use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use collator::{Collate, LinkCollate};

use crate::BuildContext;

#[derive(Clone)]
pub struct Page {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Page {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let template = h.template().ok_or_else(|| {
            RenderError::new("Type error in `page`, block template expected")
        })?;

        // Indicates that a page *must* be located, default is `true`
        let required = h
            .hash_get("required")
            .map(|v| v.value())
            .or(Some(&json!(true)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `page` helper, hash parameter `rewuired` must be a boolean",
            ))?;

        // The href or file system path
        let href_or_path = h.params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `page`, expected parameter at index 0",
                )
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `page`, expected string parameter at index 0",
                )
            })?.to_string();

        let collation = self.context.collation.read().unwrap();
        let normalized_href = collation.normalize(&href_or_path);
        let page_path = if let Some(page_path) = collation.get_link(&normalized_href) {
            page_path.to_path_buf() 
        } else {
            PathBuf::from(&href_or_path)
        };

        if let Some(page_lock) = collation.resolve(&page_path) {
            let block_context = BlockContext::new();
            rc.push_block(block_context);
            let page = page_lock.read().unwrap();
            if let Some(ref mut block) = rc.block_mut() {
                block.set_base_value(json!(&*page));
            }
            template.render(r, ctx, rc, out)?;
            rc.pop_block();
        } else {
            if required {
                return Err(RenderError::new(
                    &format!("Type error in `page`, no page found for {}", &href_or_path),
                ));
            }
        }

        Ok(())
    }
}
