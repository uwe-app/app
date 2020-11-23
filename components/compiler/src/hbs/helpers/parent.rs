use std::path::PathBuf;
use std::sync::Arc;

use bracket::{
    error::HelperError,
    helper::{Helper, HelperValue},
    render::{Render, Scope, Context, Type},
    parser::ast::Node
};
use serde_json::json;

use collator::menu;

use crate::BuildContext;

pub struct Parent {
    pub context: Arc<BuildContext>,
}

impl Helper for Parent {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(0..0)?;

        let node = ctx.assert_block(template)?;

        let base_path = rc.try_evaluate("@root/file.source", &[Type::String])?
            .as_str()
            .unwrap();

        let path = PathBuf::from(&base_path);
        let collation = self.context.collation.read().unwrap();

        if let Some(page_lock) =
            menu::parent(&self.context.options, &*collation, &path)
        {
            let scope = Scope::new();
            rc.push_scope(scope);
            let page = page_lock.read().unwrap();
            if let Some(ref mut block) = rc.scope_mut() {
                block.set_base_value(json!(&*page));
            }
            rc.template(node)?;
            rc.pop_scope();
        }

        Ok(None)
    }
}
