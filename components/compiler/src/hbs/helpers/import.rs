use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use bracket::helper::prelude::*;
use serde_json::Value;

use crate::BuildContext;
use config::markdown;

pub struct Import {
    pub context: Arc<BuildContext>,
}

impl Helper for Import {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.assert_statement(template)?;
        ctx.arity(1..1)?;

        let base_path = rc
            .try_evaluate("@root/file.template", &[Type::String])?
            .as_str()
            .unwrap();

        let mut file = Path::new(base_path).canonicalize()?.to_path_buf();

        let evaluate = self.context.options.is_markdown_file(&file);

        let mut buffer: Option<String> = None;

        let extension = self.context.config.engine().extension();

        let source = self.context.options.source.canonicalize()?.to_path_buf();

        if let Some(arg) = ctx.get(0) {
            // Handle path style import, eg: ../../docs/footer.hbs
            if let Some(value) = ctx.missing(0) {
                if let Value::String(value) = value {
                    let target = Path::new(value);
                    if target.is_absolute() {
                        if let Some(tpl) =
                            rc.get_template(&target.to_string_lossy())
                        {
                            buffer = Some(rc.buffer(tpl.node())?);
                        }
                    } else {
                        if let Some(p) = file.parent() {
                            let target = p.join(target).canonicalize().map_err(|e| {
                                HelperError::new(
                                    format!("Helper {}, could not resolve template {}", ctx.name(), value))
                            })?;
                            if let Some(tpl) =
                                rc.get_template(&target.to_string_lossy())
                            {
                                buffer = Some(rc.buffer(tpl.node())?);
                            }
                        }
                    }
                }

            // Walk parents looking for named template
            } else {
                let name =
                    ctx.try_value(arg, &[Type::String])?.as_str().unwrap();
                while let Some(p) = file.parent() {
                    let target = p.join(format!("{}.{}", name, extension));

                    if let Some(tpl) =
                        rc.get_template(&target.to_string_lossy())
                    {
                        buffer = Some(rc.buffer(tpl.node())?);
                        break;
                    }

                    // Do not go outside the site source
                    if p == source {
                        break;
                    }

                    file = p.to_path_buf();
                }
            }
        }

        if let Some(ref buffer) = buffer {
            if evaluate {
                let parsed = markdown::render(
                    &mut Cow::from(buffer),
                    &self.context.config,
                );
                rc.write(&parsed)?;
            } else {
                rc.write(buffer)?;
            }

        //rc.write(buf)?;
        } else {
            let value = ctx.get_fallback(0).unwrap().to_string();
            return Err(HelperError::new(format!(
                "Helper {}, could not resolve template {}",
                ctx.name(),
                value
            )));
        }

        Ok(None)
    }
}
