use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;

const TEXT: &str = "MADE BY UWE";
const HREF: &str = "https://uwe.app";
const TITLE: &str = "Made by Universal Web Editor";

pub struct Powered {
    pub context: Arc<BuildContext>,
}

impl Helper for Powered {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        _template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(0..0)?;

        let powered = format!(
            r#"<a href="{}" title="{}" style="text-decoration: none; font-size: 12px;">{}</a>"#,
            HREF, TITLE, TEXT
        );

        rc.write(&powered)?;
        Ok(None)
    }
}
