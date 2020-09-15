use std::sync::Arc;

use handlebars::*;

use crate::BuildContext;

#[derive(Clone)]
pub struct LiveReload {
    pub context: Arc<BuildContext>,
}

impl HelperDef for LiveReload {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if self.context.options.settings.is_live() {
            let content = livereload::embed(&self.context.config);
            out.write(&content)?;
        }
        Ok(())
    }
}
