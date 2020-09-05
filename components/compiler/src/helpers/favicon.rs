use std::path::Path;

use crate::BuildContext;
use handlebars::*;
use serde_json::json;

static DEFAULT_ICON: &str = "/assets/favicon.png";
// A transparent gif for the icon
static INLINE_ICON: &str = "data:image/gif;base64,R0lGODlhEAAQAAAAACwAAAAAAQABAAACASgAOw==";

#[derive(Clone, Copy)]
pub struct Icon<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Icon<'_> {
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
                "Type error for `favicon` helper, hash parameter `abs` must be a boolean",
            ))?;

        let mut href = h
            .hash_get("href")
            .map(|v| v.value())
            .or(Some(&json!(DEFAULT_ICON)))
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `favicon` helper, hash parameter `href` must be a string",
            ))?
            .to_string();

        let path = self
            .context
            .options
            .source
            .join(utils::url::to_path_separator(&href.trim_start_matches("/")));

        let release = self.context.options.settings.is_release();

        if !path.exists() {
            href = INLINE_ICON.to_string();
        } else {
            // Generate relative path by default
            if !abs {
                let opts = &self.context.options;
                let base_path = rc
                    .evaluate(ctx, "@root/file.source")?
                    .as_json()
                    .as_str()
                    .ok_or_else(|| {
                        RenderError::new("Type error for `file.source`, string expected")
                    })?
                    .to_string();
                let path = Path::new(&base_path);
                href = if let Ok(val) = config::link::relative(&href, path, &opts.source, opts) {
                    val
                } else {
                    return Err(RenderError::new(
                        "Type error for `favicon`, file is outside source!",
                    ));
                }
            }

            if !release {
                // Browsers will aggressively cache icons for the same host and
                // when developing locally sometimes the browser will show the
                // wrong favicon when switching projects, this should prevent that
                // by adding a random query string parameter
                href.push_str(&format!("?v={}", utils::generate_id(8)));
            }
        }

        let markup = format!("<link rel=\"icon\" href=\"{}\">", &href);
        out.write(&markup)?;
        Ok(())
    }
}
