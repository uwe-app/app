use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;

//static TEXT: &str = "UWE";
static TEXT_FULL: &str = "Made by UWE";
static HREF: &str = "https://uwe.app";
static TITLE: &str = "Made by Universal Web Editor";
static COLOR: &str = "black";
static BACKGROUND: &str = "white";
static BORDER: &str = "gray";
static PADDING: &str = "4px";
static FONT_SIZE: &str = "12px";
static BORDER_RADIUS: &str = "2px";

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

        let standard = format!(
            "text-decoration: none; color: {}, background: {}; border: 1px solid {}; padding: {}; font-size: {}; border-radius: {};",
            COLOR, BACKGROUND, BORDER, PADDING, FONT_SIZE, BORDER_RADIUS,
        );

        let powered = format!(
            r#"<a href="{}" title="{}" style="{}">{}</a>"#,
            HREF, TITLE, standard, TEXT_FULL
        );

        rc.write(&powered)?;
        Ok(None)
    }
}
