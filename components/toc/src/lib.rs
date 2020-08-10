use std::cmp::Ordering;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid heading tag name {0}")]
    InvalidTagName(String),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Eq, Clone)]
pub struct Heading {
    pub depth: usize,
    pub id: String,
    pub text: String,
}

impl PartialOrd for Heading {
    fn partial_cmp(&self, other: &Heading) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Heading {
    fn cmp(&self, other: &Heading) -> Ordering {
        self.depth.cmp(&other.depth)
    }
}

impl PartialEq for Heading {
    fn eq(&self, other: &Heading) -> bool {
        self.depth == other.depth
    }
}

impl Heading {
    pub fn parse(tag_name: &str, id: &str, text: &str) -> Result<Self> {
        let depth = if tag_name == "h1" {
            0
        } else if tag_name == "h2" {
            1
        } else if tag_name == "h3" {
            2
        } else if tag_name == "h4" {
            3
        } else if tag_name == "h5" {
            4
        } else if tag_name == "h6" {
            5
        } else {
            return Err(Error::InvalidTagName(tag_name.to_string()));
        };

        Ok(Self {
            depth,
            id: id.to_string(),
            text: text.to_string(),
        })
    }

    pub fn open(&self) -> String {
        let mut el = "<li>".to_string();
        el.push_str(&format!("<a href=\"#{}\">{}</a>", self.id, self.text));
        el
    }

    pub fn close(&self) -> &str {
        "</li>"
    }
}

#[derive(Debug)]
pub struct TableOfContents {
    pub entries: Vec<Heading>,
}

impl TableOfContents {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, tag_name: &str, id: &str, text: &str) -> Result<()> {
        let heading = Heading::parse(tag_name, id, text)?;
        self.entries.push(heading);
        Ok(())
    }

    pub fn list(&self, tag_name: &str, open: bool) -> String {
        if open {
            format!("<{}>", tag_name)
        } else {
            format!("</{}>", tag_name)
        }
    }

    pub fn to_html_string(
        &self,
        tag_name: &str,
        class_name: &str,
        from: &str,
        to: &str,
    ) -> Result<String> {
        let from = Heading::parse(from, "", "")?;
        let mut to = Heading::parse(to, "", "")?;

        if to < from {
            to = from.clone();
        }

        if self.entries.is_empty() {
            return Ok("".to_string());
        }

        let mut markup = if class_name.is_empty() {
            format!("<{}>", tag_name)
        } else {
            format!("<{} class=\"{}\">", tag_name, class_name)
        };

        let mut current: Option<&Heading> = None;
        let mut stack: Vec<&Heading> = Vec::new();

        for h in self.entries.iter() {
            if *h < from || *h > to {
                continue;
            }

            if let Some(prev) = current {
                if h > prev {
                    stack.push(h);
                    markup.push_str(&self.list(tag_name, true));
                } else if h < prev && !stack.is_empty() {
                    let _ = stack.pop();
                    markup.push_str(h.close());
                    markup.push_str(&self.list(tag_name, false));
                    markup.push_str(h.close());
                } else {
                    markup.push_str(h.close());
                }
            }

            markup.push_str(&h.open());
            current = Some(h);
        }

        if !stack.is_empty() {
            while !stack.is_empty() {
                let h = stack.pop().unwrap();
                markup.push_str(h.close());
                markup.push_str(&self.list(tag_name, false));
                markup.push_str(h.close());
            }
        } else {
            if let Some(h) = current {
                markup.push_str(h.close());
            }
        }

        markup.push_str(&format!("</{}>", tag_name));
        Ok(markup)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, Heading, Result, TableOfContents};

    #[test]
    fn empty_list() -> Result<()> {
        let mut toc = TableOfContents::new();
        let markup = toc.to_html_string("ol", "toc", "h1", "h1")?;
        let expected = "";
        assert_eq!(expected, markup);
        Ok(())
    }

    #[test]
    fn single_item() -> Result<()> {
        let mut toc = TableOfContents::new();
        toc.add("h1", "foo", "Foo");
        let markup = toc.to_html_string("ol", "toc", "h1", "h1")?;
        let expected = "<ol class=\"toc\"><li><a href=\"#foo\">Foo</a></li></ol>";
        assert_eq!(expected, markup);
        Ok(())
    }

    #[test]
    fn simple_list() -> Result<()> {
        let mut toc = TableOfContents::new();
        toc.add("h3", "foo", "Foo");
        toc.add("h3", "bar", "Bar");
        toc.add("h3", "qux", "Qux");
        let markup = toc.to_html_string("ol", "toc", "h3", "h3")?;
        let expected = "<ol class=\"toc\"><li><a href=\"#foo\">Foo</a></li><li><a href=\"#bar\">Bar</a></li><li><a href=\"#qux\">Qux</a></li></ol>";
        assert_eq!(expected, markup);
        Ok(())
    }

    #[test]
    fn simple_nested_list() -> Result<()> {
        let mut toc = TableOfContents::new();
        toc.add("h3", "foo", "Foo");
        toc.add("h4", "bar", "Bar");
        toc.add("h3", "qux", "Qux");
        let markup = toc.to_html_string("ol", "toc", "h3", "h4")?;
        let expected = "<ol class=\"toc\"><li><a href=\"#foo\">Foo</a><ol><li><a href=\"#bar\">Bar</a></li></ol></li><li><a href=\"#qux\">Qux</a></li></ol>";
        assert_eq!(expected, markup);
        Ok(())
    }

    #[test]
    fn trailing_stack_list() -> Result<()> {
        let mut toc = TableOfContents::new();
        toc.add("h3", "foo", "Foo");
        toc.add("h4", "bar", "Bar");
        toc.add("h4", "qux", "Qux");
        let markup = toc.to_html_string("ol", "toc", "h3", "h4")?;
        let expected = "<ol class=\"toc\"><li><a href=\"#foo\">Foo</a><ol><li><a href=\"#bar\">Bar</a></li><li><a href=\"#qux\">Qux</a></li></ol></li></ol>";
        assert_eq!(expected, markup);
        Ok(())
    }
}
