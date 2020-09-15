use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use crate::BuildContext;

#[derive(Clone)]
pub struct Match {
    pub context: Arc<BuildContext>,
}

impl HelperDef for Match {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // TODO: support block inner template syntax

        // Determine the href for this page
        let href = rc
            .evaluate(ctx, "@root/href")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `match` helper, unable to get page `href`",
                )
            })?
            .trim_end_matches("/")
            .to_string();

        // Get the target match destination and strip any trailing slash
        let target = h
            .params()
            .get(0)
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `match`, expected parameter at index 0",
                )
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `match`, expected string parameter at index 0",
                )
            })?
            .trim_end_matches("/");

        // Get the output to write when a match is detected
        let output = h
            .params()
            .get(1)
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `match`, expected parameter at index 1",
                )
            })?
            .value()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error in `match`, expected string parameter at index 1",
                )
            })?;

        // Determine if an exact match is required
        let exact = h
            .hash_get("exact")
            .map(|v| v.value())
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(RenderError::new(
                "Type error for `match` helper, hash parameter `exact` must be a boolean",
            ))?;

        // Perform the match logic
        let matches = (exact && href == target)
            || (!exact && target != "" && href.starts_with(&target))
            || (!exact && target == "" && href == "");

        if matches {
            out.write(&output)?;
        }

        Ok(())
    }
}
