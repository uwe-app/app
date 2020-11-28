use bracket::helper::prelude::*;
use std::sync::Arc;

use crate::BuildContext;
use collator::Collate;

pub struct Feed {
    pub context: Arc<BuildContext>,
}

impl Helper for Feed {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        let name = ctx.try_param("name", &[Type::String])?.as_str().unwrap();

        let collation = &*self.context.collation.read().unwrap();
        let base_url = self
            .context
            .options
            .get_canonical_url(&self.context.config, Some(collation.get_lang()))
            .map_err(|_e| {
                HelperError::new(
                    "Error in `feed` helper, failed to parse base URL",
                )
            })?;

        let missing_feed = HelperError::new(&format!(
            "Type error for `feed`, missing named feed {}",
            &name
        ));

        if let Some(ref feed) = self.context.config.feed {
            if let Some(ref channel) = feed.channels.get(name) {
                let channel_href =
                    channel.target.as_ref().unwrap().trim_start_matches("/");

                for feed_type in channel.types.iter() {
                    let file_name = feed_type.get_name();
                    let mime_type = feed_type.get_mime();
                    let path = format!("{}/{}", channel_href, file_name);
                    let url = base_url
                        .join(&path)
                        .map_err(|_e| {
                            HelperError::new(
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
                    rc.write(&markup)?;
                }
            } else {
                return Err(missing_feed);
            }
        } else {
            return Err(missing_feed);
        }
        Ok(None)
    }
}
