use std::path::PathBuf;
use std::sync::Arc;

use bracket::{
    error::HelperError,
    helper::{Helper, HelperValue},
    render::{Render, Scope, Context},
    parser::ast::Node
};

use serde_json::json;

use collator::menu;

use crate::BuildContext;

pub struct Components {
    pub context: Arc<BuildContext>,
}

impl Helper for Components {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        let base_path = rc
            .evaluate("@root/file.source")?
            .ok_or_else(|| {
                HelperError::new(
                    "Type error for `file.source`, expected variable",
                )
            })?
            .as_str()
            .ok_or_else(|| {
                HelperError::new(
                    "Type error for `file.source`, string expected",
                )
            })?
            .to_string();

        let node = template.ok_or_else(|| {
            HelperError::new(
                "Type error in `components`, block template expected",
            )
        })?;

        let source_path = PathBuf::from(&base_path);

        let collation = self.context.collation.read().unwrap();
        let components =
            menu::components(&self.context.options, &*collation, &source_path);
        let amount = components.len() - 1;

        let block_context = Scope::new();
        rc.push_scope(block_context);

        for (i, page) in components.iter().rev().enumerate() {
            let page = &*page.read().unwrap();
            let first = i == 0;
            let last = i == amount;
            let href = std::iter::repeat("..")
                .take(amount - i)
                .collect::<Vec<_>>()
                .join("/");

            if let Some(ref mut block) = rc.scope_mut() {
                block.set_local("first", json!(first));
                block.set_local("last", json!(last));
                block.set_local("href", json!(href));
                block.set_base_value(json!(page));
            }
            //template.render(r, ctx, rc, out)?;
            rc.template(node)?;
        }

        rc.pop_scope();

        Ok(None)
    }
}
