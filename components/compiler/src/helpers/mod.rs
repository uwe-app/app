use handlebars::*;
use std::io;

use serde_json::{Value, to_value};

use config::Page;

pub mod children;
pub mod date;
pub mod include;
pub mod json;
pub mod livereload;
pub mod markdown;
pub mod parent;
pub mod partial;
pub mod random;
pub mod slug;
pub mod url;

pub fn map_render_error(e: loader::Error) -> RenderError {
    RenderError::new(e.to_string())
}

pub struct BufferedOutput {
    buffer: String,
}

impl Output for BufferedOutput {
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.buffer.push_str(seg);
        Ok(())
    }
}

// Capture the inner template as a string.
pub fn render_buffer<'reg: 'rc, 'rc>(
    h: &Helper<'reg, 'rc>,
    r: &'reg Handlebars<'_>,
    ctx: &'rc Context,
    rc: &mut RenderContext<'reg, 'rc>,
) -> Result<String, RenderError> {
    if let Some(t) = h.template() {
        let mut buf = BufferedOutput {
            buffer: "".to_owned(),
        };
        let result = t.render(r, ctx, rc, &mut buf);
        match result {
            Ok(_) => return Ok(buf.buffer),
            Err(e) => return Err(RenderError::new(e.to_string())),
        }
    }
    Err(RenderError::new("No template for render buffer"))
}

// This dance keeps the parent context data intact
// so that the `link` helper can be called inside another
// context
pub fn with_parent_context<'rc>(
    ctx: &'rc Context,
    data: &mut Page,
) -> Result<Context, RenderError> {
    // NOTE: The old version (below) using Context::wraps() breaks data handling
    // NOTE: by serializing using from_value()
    let mut local_ctx = ctx.clone();
    let existing_data = local_ctx.data_mut();
    match existing_data {
        Value::Object(ref mut map) => {
            let mut val = to_value(&data)?;
            match val {
                Value::Object(ref mut val) => {
                    map.append(val);
                },
                _ => {}
            }
        },
        _ => {}
    }
    Ok(local_ctx)

    //let mut scope: Page = serde_json::from_value(ctx.data().clone())?;
    //scope.append(&mut data);
    //return Context::wraps(&scope);
}
