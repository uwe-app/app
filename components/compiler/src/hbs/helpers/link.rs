use std::path::Path;
use std::sync::Arc;

use bracket::helper::prelude::*;
use log::debug;
use collator::LinkCollate;
use crate::BuildContext;

pub struct Link {
    pub context: Arc<BuildContext>,
}

impl Helper for Link {

    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(1..1)?;

        let abs = rc
            .evaluate("@root/absolute")?
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let base_path = rc
            .try_evaluate("@root/file.source", &[Type::String])?.as_str().unwrap();

        let opts = &self.context.options;
        let path = Path::new(base_path);

        let mut input = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let collation = self.context.collation.read().unwrap();

        let link_config = self.context.config.link.as_ref().unwrap();
        let include_index = opts.settings.should_include_index();
        let make_relative = !abs
            && link_config.relative.is_some()
            && link_config.relative.unwrap();

        let passthrough = !input.starts_with("/")
            || input.starts_with("http:")
            || input.starts_with("https:");

        if passthrough {
            rc.write(&input)?;
            if include_index && (input == "." || input == "..") {
                rc.write("/")?;
                rc.write(config::INDEX_HTML)?;
            }
            return Ok(None);
        }

        // Strip the leading slash
        if input.starts_with("/") {
            input = input.trim_start_matches("/");
        }

        let mut base = opts.source.clone();

        if let Some(verify) = link_config.verify {
            if verify {
                //println!("Trying to verify link with input {}", input);
                //println!("Verify with input {:?}", &input);
                if !collation.find_link(&input).is_some() {
                    return Err(HelperError::new(format!(
                        "Type error for `link`, missing url {}",
                        input
                    )));
                }
            }
        }

        if let Some(ref href_path) = opts.settings.base_href {
            base.push(href_path);
            if input.starts_with(href_path) {
                input = input.trim_start_matches(href_path);
                input = input.trim_start_matches("/");
            }
        }

        let value = if make_relative {
            if let Ok(val) = opts.relative(&input, path, base) {
                val
            } else {
                return Err(HelperError::new(
                    "Type error for `link`, file is outside source!",
                ));
            }
        } else {
            format!("/{}", input)
        };

        debug!("Link {:?}", value);

        rc.write(&value)?;

        Ok(None)
    }
}
