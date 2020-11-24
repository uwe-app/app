use std::sync::Arc;
use std::borrow::Cow;
use std::path::PathBuf;

use bracket::helper::prelude::*;
use bracket::{template::Template, parser::ParserOptions};
use config::markdown as md;
use collator::{Collate, LinkCollate};

use crate::BuildContext;

fn get_front_matter_config(file: &PathBuf) -> frontmatter::Config {
    if let Some(ext) = file.extension() {
        if ext == config::HTML {
            return frontmatter::Config::new_html(false);
        }
    }
    frontmatter::Config::new_markdown(false)
}


fn render_document<'render, 'call>(
    template_path: &str,
    context: &BuildContext,
    rc: &mut Render<'render>,
    ctx: &Context<'call>,
) -> HelperValue {

    let file = PathBuf::from(template_path);
    let is_markdown = context.options.is_markdown_file(&file);

    let (content, _has_fm, _fm) =
        frontmatter::load(&file, get_front_matter_config(&file)).map_err(
            |e| {
                HelperError::new(format!(
                    "Render front matter error {} ({})",
                    template_path, e
                ))
            },
        )?;

    // TODO: use front matter line offset?
    //let options = ParserOptions::new(template_path.to_string(), 0, 0);
    //let template = Template::compile(&content, options)?;
    //let result = rc.buffer(template.node())?;

    /*
    let result =
        r.render_template_with_context(&content, ctx).map_err(|e| {
            HelperError::new(format!(
                "Render error {} ({})",
                template_path, e
            ))
        })?;

    if is_markdown {
        let parsed =
            md::render(&mut Cow::from(result), &context.config);
        rc.write(&parsed)?;
    } else {
        rc.write(&result)?;
    }
    */

    Ok(None)
}

/// Render a page block by URL path (href).
pub struct RenderPage {
    pub context: Arc<BuildContext>,
}

impl Helper for RenderPage {

    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(1..1)?;

        // The href of a page to render
        let href = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

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
                return Err(HelperError::new(&format!(
                    "Type error in `render`, no page found for {}",
                    &href
                )));
            }
        } else {
            return Err(HelperError::new(&format!(
                "Type error in `render`, no path found for {}",
                &href
            )));
        };

        render_document(&template_path, &self.context, rc, ctx)
    }
}

/// Render the page content for a layout document.
pub struct Block {
    pub context: Arc<BuildContext>,
}

impl Helper for Block {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        let template_path = rc.try_evaluate("@root/file.template", &[Type::String])?
            .as_str().unwrap().to_string();
        render_document(&template_path, &self.context, rc, ctx)
    }
}

/// Render a document layout.
pub struct Document {
    pub context: Arc<BuildContext>,
}

impl Helper for Document {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        let standalone = rc
            .evaluate("@root/standalone")?
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if standalone {
            let block = Block {context: Arc::clone(&self.context)};
            return block.call(rc, ctx, template);
        }

        let layout = rc
            .try_evaluate("@root/layout", &[Type::String])?.as_str().unwrap();

        // TODO: get the template and render it!
        if let Some(tpl) = rc.get_template(&layout) {
            rc.template(tpl.node())?;
        }

        Ok(None)
    }
}
