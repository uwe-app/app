use std::sync::Arc;

use bracket::helper::prelude::*;
use serde_json::json;

use crate::BuildContext;

pub struct Match {
    pub context: Arc<BuildContext>,
}

impl Helper for Match {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(2..2)?;

        // TODO: support block inner template syntax

        // Determine the href for this page
        let href = rc
            .try_evaluate("@root/href", &[Type::String])?
            .as_str()
            .unwrap()
            .trim_end_matches("/")
            .to_string();

        // Get the target match destination and strip any trailing slash
        let target = ctx
            .try_get(0, &[Type::String])?
            .as_str()
            .unwrap()
            .trim_end_matches("/");

        // Get the output to write when a match is detected
        let output = ctx.try_get(1, &[Type::String])?.as_str().unwrap();

        // Determine if an exact match is required
        let exact = ctx
            .param("exact")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                "Type error for `match` helper, hash parameter `exact` must be a boolean",
            ))?;

        // Perform the match logic
        let matches = (exact && href == target)
            || (!exact && target != "" && href.starts_with(&target))
            || (!exact && target == "" && href == "");

        if matches {
            rc.write(&output)?;
        }

        Ok(None)
    }
}
