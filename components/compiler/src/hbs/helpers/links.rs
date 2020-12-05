use std::path::Path;
use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;
use config::tags::link::LinkTag;

pub struct Links {
    pub context: Arc<BuildContext>,
}

impl Helper for Links {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        // Make links absolute (passthrough)
        let abs = rc
            .evaluate("@root/absolute")?
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(links) =
            rc.evaluate("@root/links")?.and_then(|v| v.as_array())
        {
            let mut tags: Vec<LinkTag> = Vec::new();

            // Collect the links into link tags
            links.iter().try_for_each(|link| {
                match serde_json::from_value::<LinkTag>(link.clone()) {
                    Ok(tag) => tags.push(tag),
                    Err(_) => {
                        return Err(HelperError::new(
                            "Invalid link tag encountered",
                        ))
                    }
                }
                Ok(())
            })?;

            // Convert to relative paths if necessary
            let tags = if abs {
                tags
            } else {
                let opts = &self.context.options;
                let base_path = rc
                    .try_evaluate("@root/file.source", &[Type::String])?
                    .as_str()
                    .unwrap();

                let path = Path::new(base_path);
                tags.iter()
                    .cloned()
                    .map(|mut tag| {
                        let src = tag.source().to_string();
                        tag.set_source(
                            opts.relative(&src, path, &opts.source).unwrap(),
                        );
                        tag
                    })
                    .collect()
            };

            for link in tags {
                rc.write(&link.to_string())?;
                rc.write("\n")?;
            }
        }

        Ok(None)
    }
}
