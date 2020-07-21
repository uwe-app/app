use std::path::Path;

use handlebars::*;
use log::debug;

use config::FileInfo;

use crate::lookup;
use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct Link<'a> {
    pub context: &'a BuildContext
}

impl HelperDef for Link<'_> {

    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let base_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file.source`, string expected"))?
            .to_string();

        let types = self.context.options.settings.types.as_ref().unwrap();

        let opts = &self.context.options;
        let path = Path::new(&base_path);

        let mut input: String = "".to_string();

        if let Some(p) = h.params().get(0) {
            let link_config = self.context.config.link.as_ref().unwrap();
            let include_index = opts.settings.should_include_index();

            if !p.is_value_missing() {
                input = p.value().as_str().unwrap_or("").to_string();
            }

            if input.is_empty() {
                return Err(RenderError::new(
                    "Type error for `link`, expected string parameter",
                ));
            }

            // Check config first
            let enabled = link_config.relative.is_some() && link_config.relative.unwrap();
            let passthrough = !enabled
                || !input.starts_with("/")
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
                input = input.trim_start_matches("/").to_owned();
            }

            let mut base = opts.source.clone();

            if let Some(verify) = link_config.verify {
                if verify {
                    //println!("Verify with input {:?}", &input);
                    if !lookup::exists(self.context, &input) {
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
                    input = input.trim_start_matches(href_path).to_owned();
                    input = input.trim_start_matches("/").to_owned();
                }
            }

            if let Ok(rel) = path.strip_prefix(base) {
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

                debug!("Link {:?}", value);

                out.write(&value)?;
            } else {
                return Err(RenderError::new(
                    "Type error for `link`, file is outside source!",
                ));
            }
        } else {
            return Err(RenderError::new(
                "Type error for `link`, expected string parameter",
            ));
        }
        Ok(())
    }
}
