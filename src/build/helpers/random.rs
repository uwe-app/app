use handlebars::*;

//use crate::utils;
//use super::render_buffer;

#[derive(Clone, Copy)]
pub struct Random;

impl HelperDef for Random{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {


        Ok(())
    }
}

