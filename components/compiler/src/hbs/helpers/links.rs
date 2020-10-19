use std::path::Path;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;
use config::tags::link::LinkTag;

#[derive(Clone)]
pub struct Links {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Links {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
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

        //let links = rc
            //.evaluate(ctx, "@root/links")?
            //.as_json()
            //.as_array()
            //.unwrap_or(vec![]);

        // List of page specific links
        let links = ctx
            .data()
            .as_object()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `links` helper, invalid page data",
                )
            })
            .unwrap()
            .get("links")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `links` helper, expected an array of links",
                )
            })?;

        let links = links
            .iter()
            .map(|item| {
                let tag: LinkTag = serde_json::from_value(item.clone()).unwrap();
                tag
            })
            .collect::<Vec<_>>();

        // Convert to relative paths if necessary
        let links = if abs {
            links
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

            links
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

        for link in links {
            out.write(&link.to_string())?;
        }

        Ok(())
    }
}
