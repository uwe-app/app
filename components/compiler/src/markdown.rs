use pulldown_cmark::{html, Options as MarkdownOptions, Parser};

pub fn render_markdown_string(content: &str) -> String {
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
