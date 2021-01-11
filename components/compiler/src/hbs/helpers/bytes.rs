use bracket::helper::prelude::*;
use serde_json::Value;

use human_bytes::human_bytes;

pub struct Bytes;

impl Helper for Bytes {
    fn call<'render, 'call>(
        &self,
        _rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.assert_statement(template)?;
        ctx.arity(1..1)?;

        let size = ctx.try_get(0, &[Type::Number])?;
        if let Value::Number(num) = size {
            if let Some(size) = num.as_u64() {
                return Ok(Some(Value::String(human_bytes(size as f64))));
            } else {
                return Err(
                    HelperError::new(
                        "Type error for `bytes`, parameter must be an unsigned integer"));
            }
        }

        Ok(None)
    }
}
