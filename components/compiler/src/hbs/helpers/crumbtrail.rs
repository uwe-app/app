use std::path::PathBuf;
use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;
use collator::menu;
use serde_json::json;

pub struct Crumbtrail {
    pub context: Arc<BuildContext>,
}

impl Helper for Crumbtrail {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(0..0)?;

        let base_path = rc
            .try_evaluate("@root/file.source", &[Type::String])?
            .as_str()
            .unwrap()
            .to_string();

        let node = ctx.assert_block(template)?;

        let source_path = PathBuf::from(&base_path);

        let collation = self.context.collation.read().unwrap();
        let components =
            menu::components(&self.context.options, &*collation, &source_path);
        let amount = components.len() - 1;

        rc.push_scope(Scope::new());

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
            rc.template(node)?;
        }

        rc.pop_scope();

        Ok(None)
    }
}
