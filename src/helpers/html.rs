use std::io;
use handlebars::*;

use crate::utils;

pub struct BufferedOutput {
    buffer: String,
}

impl Output for BufferedOutput {
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.buffer.push_str(seg);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct Element;

impl HelperDef for Element{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let params = h.params(); 

        // TODO: error on element that may not contain children

        if let Some(inner) = h.template() {
            if params.len() > 0 {
                if let Some(tag_name) = params.get(0) {
                    if let Some(name) = tag_name.value().as_str() {
                        let has_attrs = params.get(1).is_some();

                        if has_attrs {
                            out.write(&format!("<{}", name))?;
                        } else {
                            out.write(&format!("<{}>", name))?;
                        }

                        if has_attrs {
                            if let Some(attrs) = params.get(1) {
                                if let Some(att) = attrs.value().as_object() {
                                    for (k, v) in att {
                                        if let Some(s) = v.as_str() {
                                            out.write(&format!(" {}=\"{}\"", k, s))?;
                                        }
                                    }
                                }
                            }
                        }

                        // Capture the inner template as a string
                        let mut buf = BufferedOutput{buffer: "".to_owned()};
                        inner.render(r, ctx, rc, &mut buf)?;

                        // TODO: check file is markdown
                        let result = utils::render_markdown_string(&buf.buffer);

                        out.write(&result)?;
                        out.write(&format!("</{}>", name))?;
                    }
                }
            }
        }

        Ok(())
    }
}

