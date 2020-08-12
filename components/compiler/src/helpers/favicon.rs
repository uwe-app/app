use handlebars::*;
use serde_json::json;
use crate::BuildContext;

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
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let mut href = h
            .hash_get("href")
            .map(|v| v.value())
            .or(Some(&json!(DEFAULT_ICON)))
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `favicon` helper, hash parameter `href` must be a string",
            ))?
            .to_string();

        let path = self.context.options.source.join(
            utils::url::to_path_separator(&href.trim_start_matches("/")));

        let release = self.context.options.settings.is_release();

        // FIXME: support generating relative path by default

        if !path.exists() {
            href = INLINE_ICON.to_string(); 
        } else if !release {
            // Browsers will aggressively cache icons for the same host and 
            // when developing locally sometimes the browser will show the 
            // wrong favicon when switching projects, this should prevent that
            // by adding a random query string parameter
            href.push_str(&format!("?v={}", utils::generate_id(8)));
        }

        let markup = format!("<link rel=\"icon\" href=\"{}\">", &href);
        out.write(&markup)?;
        Ok(())
    }
}
