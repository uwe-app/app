use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;

use collator::{Collate, LinkCollate};
use config::markdown;

use crate::BuildContext;

fn get_front_matter_config(file: &PathBuf) -> frontmatter::Config {
    if let Some(ext) = file.extension() {
        if ext == config::HTML {
            return frontmatter::Config::new_html(false);
        }
    }
    frontmatter::Config::new_markdown(false)
}

#[derive(Clone)]
pub struct Partial {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Partial {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // The href of a page to render
        let href = h.params().get(0);

        let template_path = if let Some(href) = href {
            let href = href
                .value()
                .as_str()
                .ok_or_else(|| {
                    RenderError::new(
                        "Type error in `partial`, expected string parameter at index 0",
                    )
                })?.to_string();

            let collation = self.context.collation.read().unwrap();
            let normalized_href = collation.normalize(&href);
            if let Some(page_path) = collation.get_link(&normalized_href) {
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
                        "Type error in `partial`, no page found for {}",
                        &href
                    )));
                }
            } else {
                return Err(RenderError::new(&format!(
                    "Type error in `partial`, no path found for {}",
                    &href
                )));
            }
        } else {
            rc.evaluate(ctx, "@root/file.template")?
                .as_json()
                .as_str()
                .ok_or_else(|| {
                    RenderError::new(
                        "Type error for `file.template`, string expected",
                    )
                })?
                .to_string()
        };

        let file = PathBuf::from(&template_path);

        let is_markdown = self.context.options.is_markdown_file(&file);

        let (content, _has_fm, _fm) =
            frontmatter::load(&file, get_front_matter_config(&file)).map_err(
                |e| {
                    RenderError::new(format!(
                        "Partial front matter error {} ({})",
                        &template_path, e
                    ))
                },
            )?;

        //let result = r.render_template(&content, ctx.data()).map_err(|e| {
        let result =
            r.render_template_with_context(&content, ctx).map_err(|e| {
                RenderError::new(format!(
                    "Partial error {} ({})",
                    &template_path, e
                ))
            })?;
        //.map_err(|e| RenderError::new(format!("{}", e)))?;

        if is_markdown {
            let parsed =
                markdown::render(&mut Cow::from(result), &self.context.config);
            out.write(&parsed)?;
        } else {
            out.write(&result)?;
        }

        Ok(())
    }
}
