use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use bracket::helper::prelude::*;
use serde_json::{json, Value};

use config::markdown;
use crate::BuildContext;

pub struct Markdown {
    pub context: Arc<BuildContext>,
}

impl Helper for Markdown {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        ctx.arity(0..1)?;

        let source_path = rc
            .try_evaluate("@root/file.source", &[Type::String])?
            .as_str()
            .unwrap()
            .to_string();

        let mut buffer = String::new();

        let mut evaluate = ctx
            .param("render")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                format!("Type error for `{}` helper, hash parameter `render` must be a boolean", ctx.name()),
            ))?;

        // Parsing from block element
        if let Some(node) = template {
            buffer = rc.buffer(node)?;
        // Parse from parameters
        } else {
            if let Some(arg) = ctx.get(0) {
                // Handle path style partial template lookup {md partial}
                if let Some(value) = ctx.missing(0) {
                    if let Value::String(value) = value {
                        let partial_name = value.to_string();
                        if let Some(tpl) = rc.get_template(&partial_name) {
                            buffer = rc.buffer(tpl.node())?; 
                        } else {
                            return Err(HelperError::new(
                                format!(
                                    "Type error for `{}` helper, unable to find partial '{}'",
                                    ctx.name(), partial_name),
                            ));
                        }
                    }
                } else {
                    let param = ctx.try_value(arg, &[Type::String])?.as_str().unwrap();
                    buffer = param.to_string();
                }
            }
        }

        if !evaluate {
            let source_buf = PathBuf::from(&source_path);
            evaluate = !self.context.options.is_markdown_file(&source_buf);
        }

        //println!("md: {:?}", &source_path);
        //println!("md: {:?}", evaluate);
        //println!("md: {:?}", &buffer);

        if evaluate {
            let parsed = markdown::render(
                &mut Cow::from(buffer),
                &self.context.config,
            );
            rc.write(&parsed)?;
        } else {
            rc.write(&buffer)?;
        }

        Ok(None)
    }
}
