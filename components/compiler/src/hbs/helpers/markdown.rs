use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use bracket::helper::prelude::*;
use serde_json::json;

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

        let source_path = rc
            .try_evaluate("@root/file.source", &[Type::String])?.as_str().unwrap();
            //.as_json()
            //.as_str()
            //.ok_or_else(|| {
                //HelperError::new(
                    //"Type error in `md` for `file.source`, string expected",
                //)
            //})?
            //.to_string();
        //

        //let mut buf = BufferedOutput {
            //buffer: String::new(),
        //};

        let mut buffer = String::new();

        let mut evaluate = ctx
            .param("render")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                "Type error for `md` helper, hash parameter `render` must be a boolean",
            ))?;

        // Parsing from block element
        if let Some(node) = template {
            buffer = rc.buffer(node)?;
        // Parse from parameters
        } else {
            if let Some(path_json) = ctx.get(0) {
                // Handle path style partial template lookup {md partial}
                //if path_json.is_value_missing() {
                    //if let Some(ref path) = path_json.relative_path() {
                        //let template = r.get_template(path).ok_or(HelperError::new(format!(
                            //"Type error for `md` helper, no template found for {}",
                            //path
                        //)))?;
                        //template.render(r, ctx, rc, &mut buf)?;
                    //} else {
                        //return Err(HelperError::new(
                            //"Type error for `md` helper, unable to determine relative path",
                        //));
                    //}
                //} else {
                    let param = ctx
                        .get(0)
                        .ok_or(HelperError::new(
                            "Type error for `md` helper, failed to get parameter",
                        ))?
                        .as_str()
                        .ok_or(HelperError::new(
                            "Type error for `md` helper, parameter should be a string",
                        ))?;

                    buffer = param.to_string();

                    //println!("Got inline string buffer {:?}", &param);
                //}
            }
        }

        //println!("md: {:?}", template_name);
        //println!("md: {:?}", evaluate);
        //println!("md: {:?}", &buf.buffer);

        if !evaluate {
            let source_buf = PathBuf::from(&source_path);
            evaluate = !self.context.options.is_markdown_file(&source_buf);
        }

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
