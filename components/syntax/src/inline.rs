use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Theme};
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;

use syntect::html::{
    append_highlighted_html_for_styled_line, IncludeBackground,
};
use syntect::util::LinesWithEndings;

//
// Lifted from syntect as we want to use `code` not `pre`
//

fn start_highlighted_html_snippet(t: &Theme) -> (String, Color) {
    let c = t.settings.background.unwrap_or(Color::WHITE);
    (
        format!(
            "<code style=\"background-color:#{:02x}{:02x}{:02x};\">",
            c.r, c.g, c.b
        ),
        c,
    )
}

pub fn highlighted_html_for_string(
    s: &str,
    ss: &SyntaxSet,
    syntax: &SyntaxReference,
    theme: &Theme,
) -> String {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let (mut output, bg) = start_highlighted_html_snippet(theme);

    for line in LinesWithEndings::from(s) {
        let regions = highlighter.highlight(line, ss);
        append_highlighted_html_for_styled_line(
            &regions[..],
            IncludeBackground::IfDifferent(bg),
            &mut output,
        );
    }
    output.push_str("</code>\n");
    output
}
