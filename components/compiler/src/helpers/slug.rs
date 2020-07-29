use handlebars::*;

#[derive(Clone, Copy)]
pub struct Slug;

impl HelperDef for Slug {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let value = h.params().get(0)
            .ok_or_else(|| RenderError::new("Type error in `slug`, expected parameter"))?
            .value()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error in `slug`, expected string parameter"))?;

        out.write(&slug::slugify(&value))?;

        Ok(())
    }
}
