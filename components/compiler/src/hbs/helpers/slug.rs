use bracket::helper::prelude::*;
pub struct Slug;

impl Helper for Slug {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..1)?;

        let value = ctx.try_get(0, &[Type::String])?.as_str().unwrap();
        rc.write(&slug::slugify(value))?;
        Ok(None)
    }
}
