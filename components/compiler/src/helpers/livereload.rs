use handlebars::*;

#[derive(Clone, Copy)]
pub struct LiveReload;

impl HelperDef for LiveReload {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let runtime = runtime::runtime().read().unwrap();
        if runtime.options.settings.is_live() {
            let content = livereload::embed(&runtime.config);
            out.write(&content)?;
        }
        Ok(())
    }
}
