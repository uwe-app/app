use std::borrow::Cow;

use crate::Config;

use pulldown_cmark::{html, Event, Options, Parser};

/// Get a markdown parser for the given source.
pub fn parser<'a>(content: &'a mut Cow<str>) -> Parser<'a> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    Parser::new_ext(content, options)
}

/// Covnert markdown to HTML.
pub fn html<'a, I>(iter: I) -> String
where
    I: Iterator<Item = Event<'a>>,
{
    let mut markup = String::new();
    html::push_html(&mut markup, iter);
    markup
}

/// Render markdown to HTML appending a link catalog if necessary.
pub fn render(content: &mut Cow<str>, config: &Config) -> String {
    if let Some(ref links) = config.link {
        if let Some(ref catalog_content) = links.catalog_content {
            content.to_mut().push('\n');
            content.to_mut().push_str(catalog_content);
        }
    }
    html(parser(content))
}
