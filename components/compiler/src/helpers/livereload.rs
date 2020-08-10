use handlebars::*;

use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct LiveReload<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for LiveReload<'_> {
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
