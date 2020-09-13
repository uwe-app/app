use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;

use crate::markdown::render_markdown;
use crate::BuildContext;

use super::is_markdown_template;

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
        _h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let template_path = rc
            .evaluate(ctx, "@root/file.template")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.template`, string expected",
                )
            })?
            .to_string();

        let file = PathBuf::from(&template_path);

        let is_markdown = is_markdown_template(
            &self.context.options, ctx, rc, Some(file.clone()))?;

        let (content, _has_fm, _fm) =
            frontmatter::load(&file, get_front_matter_config(&file)).map_err(
                |e| {
                    RenderError::new(format!(
                        "Partial front matter error {} ({})",
                        &template_path, e
                    ))
                },
            )?;

        let result = r.render_template(&content, ctx.data()).map_err(|e| {
            RenderError::new(format!("Partial error {} ({})", &template_path, e))
        })?;
        //.map_err(|e| RenderError::new(format!("{}", e)))?;

        if is_markdown {
            let parsed = render_markdown(
                &mut Cow::from(result),
                &self.context.config,
            );
            out.write(&parsed)?;
        } else {
            out.write(&result)?;
        }

        Ok(())
    }
}
