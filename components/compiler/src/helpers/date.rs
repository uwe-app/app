use handlebars::*;

use chrono::{DateTime, Local, Utc};
use serde_json::from_value;

#[derive(Clone, Copy)]
pub struct DateFormat;

impl HelperDef for DateFormat {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Use local=true to convert to local timezone
        //
        // "%a %b %e %Y"

        // TODO: support format shortcuts for common formats
        // TODO: support locale aware date/time formats

        let dt = h.param(0).map(|v| v.value()).ok_or(RenderError::new(
            "Type error for `date`, first parameter must be datetime",
        ));

        let fmt = h.param(1).map(|v| v.value()).ok_or(RenderError::new(
            "Type error for `date`, second parameter must be format string",
        ));

        let local = h.hash_get("local").map(|v| v.value());

        if let Ok(dt) = dt {
            let date: DateTime<Utc> = from_value(dt.clone())?;
            if let Ok(fmt) = fmt {
                if let Some(fmt) = fmt.as_str() {
                    let format = if let Some(_) = local {
                        let converted: DateTime<Local> = DateTime::from(date);
                        converted.format(fmt).to_string()
                    } else {
                        date.format(fmt).to_string()
                    };
                    out.write(&format)?;
                }
            }
        }

        Ok(())
    }
}
