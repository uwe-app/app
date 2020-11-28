use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;
use serde_json::json;

pub struct Embed {
    pub context: Arc<BuildContext>,
}

impl Helper for Embed {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        // The identifier for which search index to use
        let id = ctx
            .param("id")
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `search` helper, hash parameter `id` must be a string",
            ))?
            .to_string();

        // Are we writing the script? Otherwise we print the embed markup.
        let script = ctx
            .param("script")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                "Type error for `search` helper, hash parameter `script` must be a boolean",
            ))?;

        // Customize the class for the embed wrapper element (embed)
        let class = ctx
            .param("class")
            .or(Some(&json!("search-wrapper")))
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `search` helper, hash parameter `class` must be a string",
            ))?
            .to_string();

        // Set the search input placeholder (embed)
        let placeholder = ctx
            .param("placeholder")
            .or(Some(&json!("Keywords")))
            .and_then(|v| v.as_str())
            .ok_or(HelperError::new(
                "Type error for `search` helper, hash parameter `placeholder` must be a string",
            ))?
            .to_string();

        // This helper is conditional on the search config so it
        // is safe to unwrap
        let search = self.context.config.search.as_ref().unwrap();

        let search_item = search.items.get(&id);
        if search_item.is_none() {
            return Err(HelperError::new(format!(
                "Type error for `search` helper, settings for `{}` search index not found",
                &id
            )));
        }

        let search_config = search_item.unwrap();

        let js = search.js.as_ref().unwrap().to_string();
        let wasm = search.wasm.as_ref().unwrap().to_string();

        let id = search_config.id.as_ref().unwrap().to_string();
        let results = search_config.results.as_ref().unwrap();
        let excerpt_buffer = search_config.excerpt_buffer.as_ref().unwrap();
        let excerpts_per_result =
            search_config.excerpts_per_result.as_ref().unwrap();

        let index_url = search_config.index.as_ref().unwrap();
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

            format!(
                "<script src=\"{}\"></script><script>{}</script>",
                js, inline
            )
        } else {
            format!(
                "<div class=\"{}\">
                  <input data-search=\"{}\" placeholder=\"{}\" class=\"search-input\">
                  <div data-search=\"{}-output\" class=\"search-output\"></div>
                </div>",
                &class, &id, &placeholder, &id
            )
        };
        rc.write(&markup)?;
        Ok(None)
    }
}
