use bracket::helper::prelude::*;
use serde_json::json;

pub struct Count;

impl Helper for Count {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(0..0)?;

        // Indicate the user wants to print the reading time derived
        // from the `avg`
        let time = ctx
            .param("time")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                "Type error for `words` helper, hash parameter `time` must be a boolean",
            ))?;

        let avg = ctx
            .param("avg")
            .or(Some(&json!(250)))
            .and_then(|v| v.as_u64())
            .ok_or(HelperError::new(
                "Type error for `words` helper, hash parameter `avg` must be a positive integer",
            ))?;

        // The average words per minute is between 200-250 so anything less
        // than this value is a bit crazy. Also this helps to avoid a divide
        // by zero panic.
        if avg < 100 {
            return Err(HelperError::new(
                "Type error for `words` helper, the `avg` value must be >= 100",
            ));
        }

        let el = if time {
            // Print reading time using the passed average
            format!("<words data-avg=\"{}\" />", &avg)
        } else {
            // Print word count directly
            format!("<words />")
        };

        rc.write(&el)?;
        Ok(None)
    }
}
