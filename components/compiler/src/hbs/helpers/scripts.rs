use std::path::Path;
use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;
use config::script::ScriptAsset;
use serde_json::Value;

pub struct Scripts {
    pub context: Arc<BuildContext>,
}

impl Helper for Scripts {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        _ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        // Make links absolute (passthrough)
        let abs = rc
            .evaluate("@root/absolute")?
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap();

        // List of page specific scripts
        let scripts = rc
            .data()
            .as_object()
            .ok_or_else(|| {
                HelperError::new(
                    "Type error for `scripts` helper, invalid page data",
                )
            })
            .unwrap()
            .get("scripts")
            .and_then(|v| v.as_array());

        // Get page-level scripts
        let scripts = if let Some(scripts) = scripts {
            scripts
                .iter()
                .map(|v| {
                    serde_json::from_value::<ScriptAsset>(v.clone()).unwrap()
                })
                .collect()
        } else {
            vec![]
        };

        // Convert to relative paths if necessary
        let scripts = if abs {
            scripts
        } else {
            let opts = &self.context.options;
            let base_path = rc
                .try_evaluate("@root/file.source", &[Type::String])?
                .as_str()
                .unwrap();

            let path = Path::new(&base_path);

            scripts
                .iter()
                .map(|script| {
                    let mut tag = script.clone().to_tag();
                    if let Some(ref src) = script.source() {
                        tag.src = Some(
                            opts.relative(src, path, &opts.source).unwrap(),
                        );
                    }
                    ScriptAsset::Tag(tag)
                })
                .collect()
        };

        // Partition so we are certain that inline
        // scripts are always rendered last.
        let (inline, scripts): (Vec<ScriptAsset>, Vec<ScriptAsset>) =
            scripts.into_iter().partition(|s| s.source().is_none());

        for script in scripts {
            rc.write(&script.to_string())?;
        }
        for script in inline {
            rc.write(&script.to_string())?;
        }

        // Render block inline scripts
        if let Some(node) = template {
            rc.write("<script>")?;
            rc.template(node)?;
            rc.write("</script>")?;

            // Render `noscript` on the inverse
            if let Some(node) = rc.inverse(node)? {
                rc.write("<noscript>")?;
                rc.template(node)?;
                rc.write("</noscript>")?;
            }
        }

        Ok(None)
    }
}
