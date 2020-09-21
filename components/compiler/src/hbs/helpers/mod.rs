use std::io;
use std::path::PathBuf;

use handlebars::*;

use config::RuntimeOptions;

pub mod author;
pub mod bookmark;
pub mod components;
pub mod date;
pub mod favicon;
pub mod feed;
pub mod include;
pub mod json;
pub mod link;
pub mod livereload;
pub mod markdown;
pub mod matcher;
pub mod menu;
pub mod page;
pub mod parent;
pub mod partial;
pub mod random;
pub mod scripts;
pub mod search;
pub mod sibling;
pub mod slug;
pub mod styles;
pub mod toc;
pub mod word;

pub struct BufferedOutput {
    buffer: String,
}

impl Output for BufferedOutput {
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.buffer.push_str(seg);
        Ok(())
    }
}

/// Determine if the template for this page
/// indicates a markdown context.
pub fn is_markdown_template<'reg: 'rc, 'rc>(
    options: &RuntimeOptions,
    ctx: &'rc Context,
    rc: &mut RenderContext<'reg, 'rc>,
    file: Option<PathBuf>,
) -> std::result::Result<bool, RenderError> {
    let file = if let Some(file) = file {
        file
    } else {
        let template_path = rc
            .evaluate(ctx, "@root/file.template")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.template`, string expected",
                )
            })?
            .to_string();
        PathBuf::from(&template_path)
    };

    Ok(options.is_markdown_file(&file))
}
