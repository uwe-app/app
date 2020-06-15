use handlebars::*;

use chrono::{DateTime, Utc};
use serde_json::{json, from_value};

use super::map_render_error;

#[derive(Clone, Copy)]
pub struct DateFormat;

impl HelperDef for DateFormat{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        // TODO: support local=true to convert to local timezone
        // TODO: support format shortcuts for common formats

        let date_err = Err(
            RenderError::new(
                "Type error for `date`, first parameter must be Utc date"));

        let format_err = Err(
            RenderError::new(
                "Type error for `date`, second parameter must be string format"));

        if let Some(dt) = h.params().get(0) {
            if !dt.is_value_missing() {
                let value = dt.value(); 
                let date: DateTime<Utc> = from_value(value.clone())?;
                if let Some(fmt) = h.params().get(1) {
                    if !fmt.is_value_missing() {
                        let value = fmt.value(); 
                        if let Some(ref fmt) = value.as_str() {
                            let format = date.format(fmt).to_string();
                            out.write(&format)?;
                        }

                    } else {
                        return format_err; 
                    }
                }
            } else {
                return date_err
            }
        }

        Ok(())
    }
}

