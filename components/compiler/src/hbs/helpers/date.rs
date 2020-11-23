use bracket::{
    helper::{Helper, HelperValue},
    render::{Render, Context, Type},
    parser::ast::Node
};

use chrono::{DateTime, Local, Utc};
use serde_json::from_value;

pub struct DateFormat;

impl Helper for DateFormat {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(2..2)?;

        // Use local=true to convert to local timezone
        //
        // "%a %b %e %Y"

        // TODO: support format shortcuts for common formats
        // TODO: support locale aware date/time formats

        let dt = ctx.try_get(0, &[Type::String])?;
        let fmt = ctx.try_get(1, &[Type::String])?.as_str().unwrap();

        let local = ctx.param("local");
        let date: DateTime<Utc> = from_value(dt.clone())?;
        let format = if let Some(_) = local {
            let converted: DateTime<Local> = DateTime::from(date);
            converted.format(fmt).to_string()
        } else {
            date.format(fmt).to_string()
        };
        rc.write(&format)?;

        Ok(None)
    }
}
