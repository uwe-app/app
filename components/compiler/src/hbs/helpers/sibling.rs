use bracket::helper::prelude::*;
use serde_json::json;

fn add(u: usize, i: i32) -> usize {
    if i.is_negative() {
        if u > 0 {
            u - i.wrapping_abs() as u32 as usize
        } else {
            0
        }
    } else {
        u + i as usize
    }
}

pub struct Sibling {
    pub amount: i32,
    pub name: String,
}

impl Helper for Sibling {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(2..2)?;

        let node = ctx.assert_block(template)?;

        // Indicates that an item *must* be located, default is `false`
        let required = ctx
            .param("required")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(format!(
                "Type error in `{}`, hash parameter `required` must be a boolean",
                self.name))
            )?;

        let list = ctx.try_get(0, &[Type::Array])?.as_array().unwrap();
        let current = ctx.get(1).unwrap();

        if list.len() > 1 {
            let pos = list.iter().position(|i| i == current);
            if let Some(pos) = pos {
                let next_pos = add(pos, self.amount);
                if next_pos < list.len() {
                    rc.push_scope(Scope::new());

                    if let Some(ref mut block) = rc.scope_mut() {
                        let sibling = &list[next_pos];
                        block.set_base_value(json!(sibling));
                    }

                    rc.template(node)?;
                    rc.pop_scope();
                }
            } else {
                if required {
                    return Err(HelperError::new(format!(
                        "Type error in `{}`, element is not in the array",
                        self.name
                    )));
                }
            }
        }

        Ok(None)
    }
}
