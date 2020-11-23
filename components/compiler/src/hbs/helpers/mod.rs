use std::borrow::Cow;
use std::path::PathBuf;

use handlebars::*;

use config::markdown as md;
use crate::BuildContext;

pub mod crumbtrail;
pub mod date;
pub mod document;
pub mod feed;
pub mod include;
pub mod link;
pub mod links;
pub mod markdown;
pub mod matcher;
pub mod menu;
pub mod page;
pub mod parent;
pub mod random;
pub mod scripts;
pub mod search;
pub mod sibling;
pub mod slug;
pub mod toc;
pub mod word;

pub struct BufferedOutput {
    buffer: String,
}

impl Output for BufferedOutput {
    fn write(&mut self, seg: &str) -> std::io::Result<()> {
        self.buffer.push_str(seg);
        Ok(())
    }
}

fn get_front_matter_config(file: &PathBuf) -> frontmatter::Config {
    if let Some(ext) = file.extension() {
        if ext == config::HTML {
            return frontmatter::Config::new_html(false);
        }
    }
    frontmatter::Config::new_markdown(false)
}


fn render_document<'reg: 'rc, 'rc>(
    template_path: &str,
    context: &BuildContext,
    _h: &Helper<'reg, 'rc>,
    r: &'reg Handlebars<'_>,
    ctx: &'rc Context,
    _rc: &mut RenderContext<'reg, 'rc>,
    out: &mut dyn Output,
) -> HelperResult {

    let file = PathBuf::from(&template_path);
    let is_markdown = context.options.is_markdown_file(&file);

    let (content, _has_fm, _fm) =
        frontmatter::load(&file, get_front_matter_config(&file)).map_err(
            |e| {
                RenderError::new(format!(
                    "Render front matter error {} ({})",
                    template_path, e
                ))
            },
        )?;

    let result =
        r.render_template_with_context(&content, ctx).map_err(|e| {
            RenderError::new(format!(
                "Render error {} ({})",
                template_path, e
            ))
        })?;

    if is_markdown {
        let parsed =
            md::render(&mut Cow::from(result), &context.config);
        out.write(&parsed)?;
    } else {
        out.write(&result)?;
    }

    Ok(())
}
