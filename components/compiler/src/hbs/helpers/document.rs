use std::sync::Arc;

use handlebars::*;

use crate::BuildContext;
use collator::{Collate, LinkCollate};

/// Render a page block by URL path (href).
#[derive(Clone)]
pub struct Render {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Render {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // The href of a page to render
        let href = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new("Type error in `render`, expected parameter")
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `render`, expected string parameter",
                )
            })?
            .to_string();

        let collation = self.context.collation.read().unwrap();
        let normalized_href = collation.normalize(&href);
        let template_path = if let Some(page_path) = collation.get_link(&normalized_href) {
            if let Some(page_lock) = collation.resolve(&page_path) {
                let page = page_lock.read().unwrap();
                page.file
                    .as_ref()
                    .unwrap()
                    .template
                    .to_string_lossy()
                    .into_owned()
            } else {
                return Err(RenderError::new(&format!(
                    "Type error in `render`, no page found for {}",
                    &href
                )));
            }
        } else {
            return Err(RenderError::new(&format!(
                "Type error in `render`, no path found for {}",
                &href
            )));
        };

        super::render_document(
            &template_path, &self.context, h, r, ctx, rc, out)
    }
}

/// Render the page content for a layout document.
#[derive(Clone)]
pub struct Block {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Block {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let template_path = rc.evaluate(ctx, "@root/file.template")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.template`, string expected",
                )
            })?
            .to_string();

        super::render_document(
            &template_path, &self.context, h, r, ctx, rc, out)
    }
}

/// Render a document layout.
#[derive(Clone)]
pub struct Document {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Document {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let standalone = rc
            .evaluate(ctx, "@root/standalone")?
            .as_json()
            .as_bool()
            .unwrap_or(false);

        if standalone {
            let block = Block {context: Arc::clone(&self.context)};
            return block.call(h, r, ctx, rc, out);
        }

        let layout = rc
            .evaluate(ctx, "@root/layout")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `layout`, string expected",
                )
            })?
            .to_string();

        let writer = Writer { out: Box::new(out) };
        r.render_to_write(&layout, ctx.data(), writer)?;

        Ok(())
    }
}

/// Helper to write to `dyn Output` via the `std::io::Write` trait.
pub struct Writer<'a> {
    out: Box<&'a mut dyn Output>,
}

impl<'a> std::io::Write for Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = match std::str::from_utf8(buf) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        self.out.write(s)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

