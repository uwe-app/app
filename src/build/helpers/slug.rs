use handlebars::*;
use log::debug;

#[derive(Clone, Copy)]
pub struct Slug;

impl HelperDef for Slug{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let type_err = Err(RenderError::new("Type error for `slug`, expected string parameter"));

        let mut input: String = "".to_string();
        if let Some(p) = h.params().get(0) {

            if !p.is_value_missing() {
                input = p.value().as_str().unwrap_or("").to_string();
            }

            if input.is_empty() {
                return type_err;
            }

            debug!("slug {:?}", input);

            out.write(&slug::slugify(&input))?;
        } else {
            return type_err;
        }
        Ok(())
    }
}
