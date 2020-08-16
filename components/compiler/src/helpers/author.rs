use handlebars::*;

use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct Author<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Author<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        Ok(())
    }
}
