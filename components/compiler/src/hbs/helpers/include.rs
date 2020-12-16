use bracket::helper::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::BuildContext;

pub struct Include {
    pub context: Arc<BuildContext>,
}

impl Helper for Include {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..1)?;

        let base_path = rc.current_name();

        // TODO: support embedding only certain lines only
        let mut buf = Path::new(base_path).to_path_buf();

        let source = self.context.options.source.canonicalize()?.to_path_buf();

        // NOTE: this allows quoted strings and raw paths
        if let Some(include_file) = ctx.get_fallback(0) {
            let include_file = ctx
                .try_value(include_file, &[Type::String])?
                .as_str()
                .unwrap();

            // Absolute paths are resolved relative to the site directory
            buf = if include_file.starts_with("/") {
                self.context.options.source
                    .join(include_file.trim_start_matches("/")).to_path_buf()
            } else {
                if let Some(parent) = buf.parent() {
                    buf = parent.to_path_buf();
                    buf.push(include_file);
                    buf
                } else { PathBuf::from(include_file) }
            };


            if !buf.exists() {
                return Err(HelperError::new(format!(
                    "Missing include file {}",
                    buf.display()
                )));
            }

            buf = buf.canonicalize()?;

            if !buf.starts_with(&source) {
                return Err(HelperError::new(format!(
                    "Include {} is not allowed because it is outside of the source directory {}",
                    buf.display(),
                    source.display(),
                )));
            }

            let result = utils::fs::read_string(&buf).map_err(|e| {
                HelperError::new(format!(
                    "Failed to read from include file: {}",
                    buf.display()
                ))
            })?;
            rc.write(&result)?;
        }

        Ok(None)
    }
}
