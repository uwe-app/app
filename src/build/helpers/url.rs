use std::path::Path;

use handlebars::*;
use serde_json::json;
use log::debug;

use crate::build::matcher;
use crate::utils;
use crate::BuildOptions;

#[derive(Clone, Copy)]
pub struct Relative;

impl HelperDef for Relative{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
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

            if input == "/" {
                input = "".to_string();
            }

            // Strip the leading slash
            input = input.replacen("/", "", 1);

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

            if let Ok(rel) = path.strip_prefix(opts.source) {
                //println!("got relative {:?}", p);
                //println!("got relative {:?}", input);
                //println!("got relative {:?}", path.display());
                //println!("got relative {:?}", rel.display());

                let mut parents: String = "".to_string();
                if let Some(p) = rel.parent() {
                    if opts.clean_url && matcher::is_clean(&path) {
                        parents.push_str("../");
                    }
                    for part in p.components() {
                        parents.push_str("../");
                    }
                }

                parents.push_str(&input);
                debug!("rel {:?}", parents);
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

