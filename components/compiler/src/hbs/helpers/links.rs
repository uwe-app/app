use std::path::Path;
use std::sync::Arc;

use handlebars::*;

use crate::BuildContext;
use config::tags::link::LinkTag;

#[derive(Clone)]
pub struct Links {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Links {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Make links absolute (passthrough)
        let abs = rc
            .evaluate(ctx, "@root/absolute")?
            .as_json()
            .as_bool()
            .unwrap_or(false);

        if let Some(links) = rc
            .evaluate(ctx, "@root/links")?
            .as_json()
            .as_array() {

            let mut tags: Vec<LinkTag> = Vec::new();

            // Collect the links into link tags
            links
                .iter()
                .try_for_each(|link| {
                    match serde_json::from_value::<LinkTag>(link.clone()) {
                        Ok(tag) => tags.push(tag),
                        Err(_) => {
                            return Err(RenderError::new("Invalid link tag encountered"))
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
                    .evaluate(ctx, "@root/file.source")?
                    .as_json()
                    .as_str()
                    .ok_or_else(|| {
                        RenderError::new(
                            "Type error for `file.source`, string expected",
                        )
                    })?
                    .to_string();

                let path = Path::new(&base_path);
                tags
                    .iter()
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
                out.write(&link.to_string())?;
            }
        }

        Ok(())
    }
}
