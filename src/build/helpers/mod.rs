use std::io;
use handlebars::*;
use super::Error;

pub mod html;
pub mod children;
pub mod json;
pub mod markdown;
pub mod parent;
pub mod include;

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
    rc: &mut RenderContext<'reg, 'rc>) -> Result<String, Error> {
    if let Some(t) = h.template() {
        let mut buf = BufferedOutput{buffer: "".to_owned()};
        let result = t.render(r, ctx, rc, &mut buf);
        match result {
            Ok(_) => {
                return Ok(buf.buffer)
            },
            Err(e) => return Err(Error::RenderError(e)),
        }
    }
    Err(Error::RenderError(RenderError::new("no template for render buffer")))
}

