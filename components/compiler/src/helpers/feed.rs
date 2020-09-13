use handlebars::*;
use std::sync::Arc;

use crate::BuildContext;
use collator::Collate;

#[derive(Clone)]
pub struct Feed {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Feed {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let name = h
            .hash_get("name")
            .map(|v| v.value())
            .and_then(|v| v.as_str())
            .ok_or(RenderError::new(
                "Type error for `feed` helper, hash parameter `name` must be a string",
            ))?
            .to_string();

        let collation = &*self.context.collation.read().unwrap();
        let base_url = self
            .context
            .options
            .get_canonical_url(&self.context.config, Some(collation.get_lang()))
            .map_err(|_e| {
                RenderError::new(
                    "Error in `feed` helper, failed to parse base URL",
                )
            })?;

        let missing_feed = RenderError::new(&format!(
            "Type error for `feed`, missing named feed {}",
            &name
        ));

        if let Some(ref feed) = self.context.config.feed {
            if let Some(ref channel) = feed.channels.get(&name) {
                let channel_href =
                    channel.target.as_ref().unwrap().trim_start_matches("/");

                for feed_type in channel.types.iter() {
                    let file_name = feed_type.get_name();
                    let mime_type = feed_type.get_mime();
                    let path = format!("{}/{}", channel_href, file_name);
                    let url = base_url
                        .join(&path)
                        .map_err(|_e| {
                            RenderError::new(
                                "Error in `feed` helper, failed to join URL",
                            )
                        })?
                        .to_string();

                    let markup = if let Some(ref title) = channel.title {
                        format!(
                            "<link rel=\"alternate\" title=\"{}\" type=\"{}\" href=\"{}\" />",
                            title, mime_type, url
                        )
                    } else {
                        format!(
                            "<link rel=\"alternate\" type=\"{}\" href=\"{}\" />",
                            mime_type, url
                        )
                    };
                    out.write(&markup)?;
                }
            } else {
                return Err(missing_feed);
            }
        } else {
            return Err(missing_feed);
        }
        Ok(())
    }
}
