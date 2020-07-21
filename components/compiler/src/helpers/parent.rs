use std::path::Path;

use handlebars::*;

use crate::BuildContext;

use super::map_render_error;
use super::with_parent_context;

#[derive(Clone, Copy)]
pub struct Parent<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Parent<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let base_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
            .to_string();

        let template = h.template()
            .ok_or_else(|| RenderError::new("Type error in `parent`, block template expected"))?;

        let types = self.context.options.settings.types.as_ref().unwrap();

        let path = Path::new(&base_path).to_path_buf();

        if let Some(parent) = config::resolve::resolve_parent_index(&path, types) {

            let mut data = loader::compute(&parent, &self.context.config, &self.context.options, true)
                .map_err(map_render_error)?;
            let mut local_rc = rc.clone();
            let local_ctx = with_parent_context(ctx, &mut data)?;
            template.render(r, &local_ctx, &mut local_rc, out)?;
            return Ok(());

        }
        Ok(())
    }
}
