use handlebars::*;

use serde_json::json;

#[derive(Clone, Copy)]
pub struct Count;

impl HelperDef for Count {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        // Indicate the user wants to print the reading time derived 
        // from the `avg`
        let time = h.hash_get("time")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `words` helper, hash parameter `time` must be a boolean"
            ))?;

        let avg = h.hash_get("avg")
            .map(|v| v.value())
            .or(Some(&json!(250)))
            .and_then(|v| v.as_u64())
            .ok_or(RenderError::new(
                "Type error for `words` helper, hash parameter `avg` must be a positive integer"
            ))?;

        // The average words per minute is between 200-250 so anything less
        // than this value is a bit crazy. Also this helps to avoid a divide
        // by zero panic.
        if avg < 100 {
            return Err(
                RenderError::new(
                    "Type error for `words` helper, the `avg` value must be >= 100")); 
        }

        let el = if time {
            // Print reading time using the passed average
            format!("<words data-avg=\"{}\" />", &avg)
        } else {
            // Print word count directly
            format!("<words />")
        };

        out.write(&el)?;
        Ok(())
    }
}
