use std::io;
use handlebars::*;

use serde_json::{json, Map, Value};

pub mod children;
pub mod html;
pub mod include;
pub mod json;
pub mod livereload;
pub mod markdown;
pub mod parent;
pub mod random;
pub mod slug;
pub mod url;

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
    rc: &mut RenderContext<'reg, 'rc>) -> Result<String, RenderError> {
    if let Some(t) = h.template() {
        let mut buf = BufferedOutput{buffer: "".to_owned()};
        let result = t.render(r, ctx, rc, &mut buf);
        match result {
            Ok(_) => {
                return Ok(buf.buffer)
            },
            Err(e) => return Err(RenderError::new(e.to_string())),
        }
    }
    Err(RenderError::new("no template for render buffer"))
}

// This dance keeps the parent context data intact
// so that the `link` helper can be called inside another 
// context
pub fn with_parent_context<'rc>(
    ctx: &'rc Context,
    data: &Map<String, Value>) -> Result<Context, RenderError> {

    let existing = ctx.data().as_object().unwrap();
    let mut new_data: Map<String, Value> = existing.clone();
    for (k, v) in data {
        new_data.insert(k.clone(), json!(v));
    }

    return Context::wraps(&new_data);
}

