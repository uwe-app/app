use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;

use collator::menu;

use crate::BuildContext;

use super::with_parent_context;

#[derive(Clone)]
pub struct Parent {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Parent {
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
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.source`, string expected",
                )
            })?
            .to_string();

        let template = h.template().ok_or_else(|| {
            RenderError::new("Type error in `parent`, block template expected")
        })?;

        let path = PathBuf::from(&base_path);
        let collation = self.context.collation.read().unwrap();
        if let Some(data) = menu::parent(&self.context.options, &*collation, &path) {
            let mut page = data.write().unwrap();
            //let mut page = data.clone();
            let mut local_rc = rc.clone();
            let local_ctx = with_parent_context(ctx, &mut page)?;
            template.render(r, &local_ctx, &mut local_rc, out)?;
            return Ok(());
        }
        Ok(())
    }
}
