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

        let dt = h
            .params()
            .get(0)
            .ok_or_else(|| RenderError::new("Type error in `date`, expected date parameter"))?
            .value();

        let fmt = h
            .params()
            .get(1)
            .ok_or_else(|| RenderError::new("Type error in `date`, expected format parameter"))?
            .value()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error in `date`, format must be a string"))?;

        let local = h.hash_get("local").map(|v| v.value());
        let date: DateTime<Utc> = from_value(dt.clone())?;
        let format = if let Some(_) = local {
            let converted: DateTime<Local> = DateTime::from(date);
            converted.format(fmt).to_string()
        } else {
            date.format(fmt).to_string()
        };
        out.write(&format)?;

        Ok(())
    }
}
