use std::path::PathBuf;
use std::borrow::Cow;

use handlebars::*;

use crate::markdown::render_markdown_string;
use crate::BuildContext;

fn get_front_matter_config(file: &PathBuf) -> frontmatter::Config {
    if let Some(ext) = file.extension() {
        if ext == config::HTML {
            return frontmatter::Config::new_html(false)
        } 
    }
    frontmatter::Config::new_markdown(false)
}

#[derive(Clone, Copy)]
pub struct Partial<'a> {
    pub context: &'a BuildContext
}

impl HelperDef for Partial<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let source_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
            .replace("\"", "");

        //println!("Template partial was called");
        //println!("Got base path {}", source_path);

        let types = self.context.options.settings.types.as_ref().unwrap();
        let mut evaluate = false;

        let file = PathBuf::from(&source_path);
        if let Some(ext) = file.extension() {
            let s = ext.to_string_lossy().into_owned();
            evaluate = types.markdown().contains(&s);
        }

        // TODO: handle this error
        let (content, _has_fm, _fm) =
            frontmatter::load(&file, get_front_matter_config(&file)).unwrap();

        //r.register_template_string(&source_path, content);

        let result = r.render_template(&content, ctx.data()).unwrap();

        //println!("Res {}", result);

        if evaluate {
            let parsed = render_markdown_string(&mut Cow::from(result), &self.context.config);
            out.write(&parsed)?;
        } else {
            out.write(&result)?;
        }

        Ok(())
    }
}
