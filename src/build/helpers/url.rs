use std::path::Path;

use handlebars::*;
use serde_json::json;
use log::debug;

use crate::build::matcher;
use crate::BuildOptions;

#[derive(Clone, Copy)]
pub struct Link;

impl HelperDef for Link{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let mut input: String = "".to_string();

        if let Some(p) = h.params().get(0) {
            if !p.is_value_missing() {
                input = p.value().as_str().unwrap_or("").to_string();
            }

            if input.is_empty() {
                return Err(RenderError::new("Type error for `rel`, expected string parameter")) 
            }

            let passthrough = !input.starts_with("/") || input.starts_with("http:") || input.starts_with("https:");
            if passthrough {
                out.write(&input)?;
                return Ok(())
            }

            // Strip the leading slash
            if input.starts_with("/") {
                input = input.replacen("/", "", 1);
            }

            let base_path = rc
                .evaluate(ctx, "@root/context.file")?
                .as_json()
                .as_str()
                .ok_or_else(|| RenderError::new("Type error for `file`, string expected"))?
                .replace("\"", "");

            let opts = rc
                .evaluate(ctx, "@root/context.options")?
                .as_json()
                .as_object()
                .ok_or_else(|| RenderError::new("Type error for `options`, map expected"))?
                .to_owned();

            let opts: BuildOptions = serde_json::from_value(json!(opts)).unwrap();
            let path = Path::new(&base_path).to_path_buf();

            let exists = matcher::source_exists(&opts.source, &input, opts.clean_url);

            if !exists {
                return Err(RenderError::new(format!("Type error for `link`, missing url {}", input)))
            }

            if let Ok(rel) = path.strip_prefix(opts.source) {
                let mut parents: String = "".to_string();
                if let Some(p) = rel.parent() {
                    if opts.clean_url && matcher::is_clean(&path) {
                        parents.push_str("../");
                    }
                    for _ in p.components() {
                        parents.push_str("../");
                    }
                }

                parents.push_str(&input);
                debug!("link {:?}", parents);
                out.write(&parents)?;

            } else {
                return Err(RenderError::new("Type error for `rel`, file is outside source!")) 
            }


        } else {
            return Err(RenderError::new("Type error for `rel`, expected string parameter")) 
        }
        Ok(())
    }
}

