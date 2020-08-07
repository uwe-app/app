use handlebars::*;

use serde_json::json;

use crate::BuildContext;

#[derive(Clone, Copy)]
pub struct Embed<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for Embed<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        // Are we writing the script? Otherwise we print the embed markup.
        let script = h.hash_get("script")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `search` helper, hash parameter `script` must be a boolean"
            ))?;

        // Customize the class for the embed wrapper element (embed)
        let class = h.hash_get("class")
            .map(|v| v.value())
            .or(Some(&json!("search-wrapper")))
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `search` helper, hash parameter `class` must be a string"
            ))?.to_string();

        // Set the search input placeholder (embed)
        let placeholder = h.hash_get("placeholder")
            .map(|v| v.value())
            .or(Some(&json!("Keywords")))
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `search` helper, hash parameter `placeholder` must be a string"
            ))?.to_string();

        // This helper is conditional on the search config so it
        // is safe to unwrap
        let search_config = self.context.config.search.as_ref().unwrap();
        let id = search_config.id.as_ref().unwrap().to_string();
        let js = search_config.js.as_ref().unwrap().to_string();
        let wasm = search_config.wasm.as_ref().unwrap().to_string();

        let results = search_config.results.as_ref().unwrap();
        let excerpt_buffer = search_config.excerpt_buffer.as_ref().unwrap();
        let excerpts_per_result = search_config.excerpts_per_result.as_ref().unwrap();

        let index_url = search_config.target.as_ref().unwrap();
        let markup = if script {
            let inline = format!(
                "search.register(\"{}\", \"{}\",
                    {{
                        runtime: \"{}\",
                        showProgress: true,
                        showScores: true,
                        printIndexInfo: true,
                        options: {{
                            results: {},
                            excerpt_buffer: {},
                            excerpts_per_result: {}
                        }}
                    }});",
                    &id,
                    index_url,
                    &wasm,
                    &results,
                    &excerpt_buffer,
                    &excerpts_per_result,
            );

            format!("<script src=\"{}\"></script><script>{}</script>", js, inline)

        } else {
            format!(
                "<div class=\"{}\">
                  <input data-search=\"{}\" placeholder=\"{}\" class=\"search-input\"></input>
                  <div data-search=\"{}-output\" class=\"search-output\"></div>
                </div>",
                &class,
                &id,
                &placeholder,
                &id
            )
        };
        out.write(&markup)?;
        Ok(())
    }
}
