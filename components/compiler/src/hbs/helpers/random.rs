use bracket::{
    helper::{Helper, HelperValue},
    render::{Render, Scope, Context, Type},
    parser::ast::Node
};
use rand::seq::SliceRandom;
use serde_json::json;

pub struct Random;

impl Helper for Random {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(1..1)?;

        let node = ctx.assert_block(template)?;
        let list = ctx.try_get(0, &[Type::Array])?.as_array().unwrap();

        let scope = Scope::new();
        rc.push_scope(scope);

        if let Some(entry) = list.choose(&mut rand::thread_rng()) {
            if let Some(ref mut block) = rc.scope_mut() {
                block.set_base_value(json!(entry));
            }
            rc.template(node)?;
        }

        rc.pop_scope();

        Ok(None)
    }
}
