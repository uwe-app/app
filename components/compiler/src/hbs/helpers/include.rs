use bracket::helper::prelude::*;
use std::path::Path;

pub struct Include;

impl Helper for Include {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..1)?;

        let base_path = rc
            .try_evaluate("@root/file.source", &[Type::String])?
            .as_str()
            .unwrap();

        // TODO: support embedding only certain lines only
        let mut buf = Path::new(base_path).to_path_buf();

        // NOTE: this allows quoted strings and raw paths
        if let Some(include_file) = ctx.get_fallback(0) {
            let include_file = ctx
                .try_value(include_file, &[Type::String])?
                .as_str()
                .unwrap();

            if let Some(parent) = buf.parent() {
                buf = parent.to_path_buf();
                buf.push(include_file);
                let result = utils::fs::read_string(&buf).map_err(|e| {
                    HelperError::new(format!(
                        "Failed to read from include file: {}",
                        buf.display()
                    ))
                })?;
                rc.write(&result)?;
            }
        }

        Ok(None)
    }
}
