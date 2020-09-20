use handlebars::*;
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

#[derive(Clone)]
pub struct Sibling {
    pub amount: i32,
    pub name: String,
}

impl HelperDef for Sibling {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Indicates that an item *must* be located, default is `false`
        let required = h
            .hash_get("required")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(format!(
                "Type error in `{}`, hash parameter `required` must be a boolean",
                self.name))
            )?;

        let list = h
            .params()
            .get(0)
            .ok_or(RenderError::new(format!(
                "Type error in `{}`, expected parameter at index 0",
                self.name
            )))?
            .value()
            .as_array()
            .ok_or(RenderError::new(format!(
                "Type error in `{}`, expected array parameter",
                self.name
            )))?;

        let current = h
            .params()
            .get(1)
            .ok_or(RenderError::new(format!(
                "Type error in `{}`, expected parameter at index 1",
                self.name
            )))?
            .value();

        let template = h.template().ok_or(RenderError::new(format!(
            "Type error in `{}`, block template expected",
            self.name
        )))?;

        if list.len() > 1 {
            let pos = list.iter().position(|i| i == current);
            if let Some(pos) = pos {
                let next_pos = add(pos, self.amount);
                if next_pos < list.len() {
                    let block_context = BlockContext::new();
                    rc.push_block(block_context);

                    let sibling = &list[next_pos];

                    if let Some(ref mut block) = rc.block_mut() {
                        block.set_base_value(json!(sibling));
                    }

                    template.render(r, ctx, rc, out)?;

                    rc.pop_block();
                }
            } else {
                if required {
                    return Err(RenderError::new(format!(
                        "Type error in `{}`, element is not in the array",
                        self.name
                    )));
                }
            }
        }

        Ok(())
    }
}
