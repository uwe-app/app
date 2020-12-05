use serde::{Deserialize, Serialize};
use std::fmt;

static CLASS: &str = "powered-by";
static TEXT: &str = "UWE";
static TEXT_FULL: &str = "Made by UWE";
static HREF: &str = "https://uwe.app";
static TITLE: &str = "Made by Universal Web Editor";
static COLOR: &str = "black";
static BACKGROUND: &str = "white";
static BORDER: &str = "gray";
static PADDING: &str = "4px";
static FONT_SIZE: &str = "12px";
static BORDER_RADIUS: &str = "2px";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Display {
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "relative")]
    Relative,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "hidden")]
    Hidden,
}

impl Default for Display {
    fn default() -> Self {
        Display::Fixed
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Powered {
    color: Option<String>,
    background: Option<String>,
    border: Option<String>,
    display: Option<Display>,
}

impl Powered {
    pub fn hidden(&self) -> bool {
        if let Some(Display::Hidden) = self.display {
            true
        } else { false }
    }
}

impl Default for Powered {
    fn default() -> Self {
        Powered {
            color: Some(COLOR.to_string()),
            border: Some(BORDER.to_string()),
            background: Some(BACKGROUND.to_string()),
            display: Some(Default::default()),
        }
    }
}
impl fmt::Display for Powered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let standard = format!(
            "text-decoration: none; color: {}; background: {}; border: 1px solid {}; padding: {}; font-size: {}; border-radius: {};",
            self.color.as_ref().unwrap(), self.background.as_ref().unwrap(), self.border.as_ref().unwrap(), PADDING, FONT_SIZE, BORDER_RADIUS,
        );
        write!(
            f,
            "{}",
            match self.display.as_ref().unwrap() {
                Display::Fixed => {
                    let style = format!(
                        "position: fixed; bottom: 12px; right: 24px; {}",
                        standard
                    );
                    format!(
                        r#"<a href="{}" title="{}" style="{}">{}</a>"#,
                        HREF, TITLE, style, TEXT
                    )
                }
                Display::Relative => {
                    format!(
                        r#"<div style="text-align: center; padding-bottom: 24px;"><a href="{}" title="{}" style="{}">{}</a></div>"#,
                        HREF, TITLE, standard, TEXT_FULL
                    )
                }
                Display::None => {
                    format!(r#"<a href="{}" title="{}" class="{}">{}</a>"#, HREF, TITLE, CLASS, TEXT)
                }
                Display::Hidden => { String::new() }
            }
        )
    }
}
