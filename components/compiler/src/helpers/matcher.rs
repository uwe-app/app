use std::path::Path;

use crate::BuildContext;
use handlebars::*;

#[derive(Clone, Copy)]
pub struct Match<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Match<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let base_path = rc
            .evaluate(ctx, "@root/file.target")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.target`, string expected"))?
            .to_string();

        let opts = &self.context.options;
        let path = Path::new(&base_path).to_path_buf();

        if h.params().len() != 2 && h.params().len() != 3 {
            return Err(RenderError::new(
                "Type error for `match`, two parameters expected",
            ));
        }

        let mut target = "".to_owned();
        let mut output = "".to_owned();
        let mut exact = false;

        if let Some(p) = h.params().get(0) {
            if !p.is_value_missing() {
                target = p.value().as_str().unwrap_or("").to_string();
            }
        }

        if target.ends_with("/") {
            target = target.trim_end_matches("/").to_string();
        }

        if let Some(p) = h.params().get(1) {
            if !p.is_value_missing() {
                output = p.value().as_str().unwrap_or("").to_string();
            }
        }

        if let Some(p) = h.params().get(2) {
            if !p.is_value_missing() {
                exact = p.value().as_bool().unwrap_or(true);
            }
        }

        if let Ok(rel) = path.strip_prefix(&opts.target) {
            let mut pth = "".to_string();
            pth.push('/');
            pth.push_str(&rel.to_string_lossy().into_owned());
            if pth.ends_with(config::INDEX_HTML) {
                pth = pth.trim_end_matches(config::INDEX_HTML).to_string();
            }
            if pth.ends_with("/") {
                pth = pth.trim_end_matches("/").to_string();
            }

            let matches = (exact && pth == target)
                || (!exact && target != "" && pth.starts_with(&target))
                || (!exact && target == "" && pth == "");

            if matches {
                out.write(&output)?;
            }
        }
        Ok(())
    }
}
