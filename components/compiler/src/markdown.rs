use std::borrow::Cow;

use config::Config;

use pulldown_cmark::{html, Options as MarkdownOptions, Parser};

pub fn render_markdown_string(
    content: &mut Cow<str>,
    config: &Config,
) -> String {
    if let Some(ref links) = config.link {
        if let Some(ref catalog_content) = links.catalog_content {
            content.to_mut().push('\n');
            content.to_mut().push_str(catalog_content);
        }
    }

    let mut options = MarkdownOptions::empty();
    options.insert(MarkdownOptions::ENABLE_TABLES);
    options.insert(MarkdownOptions::ENABLE_FOOTNOTES);
    options.insert(MarkdownOptions::ENABLE_STRIKETHROUGH);
    options.insert(MarkdownOptions::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(content, options);
    let mut markup = String::new();
    html::push_html(&mut markup, parser);
    markup
}
