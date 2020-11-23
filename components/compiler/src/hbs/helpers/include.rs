use bracket::{
    error::HelperError,
    helper::{Helper, HelperValue},
    render::{Render, Context, Type},
    parser::ast::Node
};

use std::path::Path;

pub struct Include;

impl Helper for Include {

    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {

        let base_path = rc.try_evaluate("@root/file.source", &[Type::String])?
            .unwrap()
            .to_string();

        //let base_path = rc
            //.evaluate(ctx, "@root/file.source")?
            //.as_json()
            //.as_str()
            //.ok_or_else(|| {
                //HelperError::new(
                    //"Type error for `file.source`, string expected",
                //)
            //})?
            //.to_string();

        // TODO: support embedding only certain lines only

        let mut buf = Path::new(&base_path).to_path_buf();

        if let Some(parent) = buf.parent() {
            buf = parent.to_path_buf();
            let val = ctx.try_get(0, &[Type::String])?.as_str().unwrap();
            //if let Some(req) = h.params().get(0) {
                // TODO: support using "value()" too?
                //if let Some(val) = req.relative_path() {
                    buf.push(val);
                    let result = utils::fs::read_string(&buf);
                    match result {
                        Ok(s) => {
                            rc.write(&s)?;
                        }
                        Err(_) => {
                            return Err(HelperError::new(format!(
                                "Failed to read from include file: {}",
                                buf.display()
                            )))
                        }
                    }
                //}
            //}
        }

        Ok(None)
    }
}
