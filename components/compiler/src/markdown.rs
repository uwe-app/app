use std::borrow::Cow;

use config::Config;

use pulldown_cmark::{html, Options, Parser};

fn get_parser<'a>(content: &'a mut Cow<str>) -> Parser<'a> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    Parser::new_ext(content, options)
}

pub fn render_markdown(content: &mut Cow<str>, config: &Config) -> String {
    if let Some(ref links) = config.link {
        if let Some(ref catalog_content) = links.catalog_content {
            content.to_mut().push('\n');
            content.to_mut().push_str(catalog_content);
        }
    }

    let parser = get_parser(content);
    let mut markup = String::new();
    html::push_html(&mut markup, parser);
    markup
}
