use handlebars::*;
use serde_json::{to_string, to_string_pretty};

#[derive(Clone)]
pub struct Debug;

impl HelperDef for Debug /*<'_>*/ {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let mut compact = false;
        let mut val = ctx.data();

        if let Some(p) = h.params().get(0) {
            val = p.value();
        }

        // Support compact flag on second parameter
        if let Some(p) = h.params().get(1) {
            if !p.is_value_missing() {
                if let Some(b) = p.value().as_bool() {
                    compact = b;
                }
            }
        }

        if compact {
            if let Ok(s) = to_string(val) {
                out.write(&s)?;
            }
        } else {
            if let Ok(s) = to_string_pretty(val) {
                out.write(&s)?;
            }
        }

        Ok(())
    }
}
