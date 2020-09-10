use std::path::Path;
use std::sync::Arc;

use handlebars::*;
use log::debug;
use serde_json::json;

//use config::FileInfo;

use crate::lookup;
use crate::BuildContext;

#[derive(Clone)]
pub struct Link {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Link {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let abs = h
            .hash_get("abs")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `link` helper, hash parameter `abs` must be a boolean",
            ))?;

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

        //let types = self.context.options.settings.types.as_ref().unwrap();

        let opts = &self.context.options;
        let path = Path::new(&base_path);

        let mut input = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new("Type error in `link`, expected parameter")
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `link`, expected string parameter",
                )
            })?;

        let link_config = self.context.config.link.as_ref().unwrap();
        let include_index = opts.settings.should_include_index();
        let make_relative = !abs
            && link_config.relative.is_some()
            && link_config.relative.unwrap();

        let passthrough = !input.starts_with("/")
            || input.starts_with("http:")
            || input.starts_with("https:");

        if passthrough {
            out.write(&input)?;
            if include_index && (input == "." || input == "..") {
                out.write("/")?;
                out.write(config::INDEX_HTML)?;
            }
            return Ok(());
        }

        // Strip the leading slash
        if input.starts_with("/") {
            input = input.trim_start_matches("/");
        }

        let mut base = opts.source.clone();

        if let Some(verify) = link_config.verify {
            if verify {
                //println!("Verify with input {:?}", &input);
                if !lookup::exists(&self.context, &input) {
                    return Err(RenderError::new(format!(
                        "Type error for `link`, missing url {}",
                        input
                    )));
                }
            }
        }

        if let Some(ref href_path) = opts.settings.base_href {
            base.push(href_path);
            if input.starts_with(href_path) {
                input = input.trim_start_matches(href_path);
                input = input.trim_start_matches("/");
            }
        }

        let value = if make_relative {
            if let Ok(val) =
                opts.relative(&input, path, base)
            {
                val
            } else {
                return Err(RenderError::new(
                    "Type error for `link`, file is outside source!",
                ));
            }
        } else {
            format!("/{}", input)
        };

        debug!("Link {:?}", value);

        out.write(&value)?;

        /*
        if let Ok(rel) = path.strip_prefix(base) {
            let value = if make_relative {
                let up = "../";
                let mut value: String = "".to_string();
                if let Some(p) = rel.parent() {
                    if opts.settings.should_rewrite_index() && FileInfo::is_clean(&path, types) {
                        value.push_str(up);
                    }
                    for _ in p.components() {
                        value.push_str(up);
                    }
                }

                value.push_str(&input);

                if include_index && (value.ends_with("/") || value == "") {
                    value.push_str(config::INDEX_HTML);
                }

                if !opts.settings.should_rewrite_index() && value == "" {
                    value = up.to_string();
                }
                value
            } else {
                format!("/{}", input)
            };

            debug!("Link {:?}", value);

            out.write(&value)?;
        } else {
            return Err(RenderError::new(
                "Type error for `link`, file is outside source!",
            ));
        }
        */

        Ok(())
    }
}
