use regex::Regex;

use crate::Result;
use crate::text::TextExtraction;

#[derive(Debug)]
pub struct TransformCache {
    // Patter for counting words
    pub words_re: Regex,

    // Extracted text.
    pub text: Option<TextExtraction>,

    // This flag is used internally to trigger syntax highlighting
    // transformations when the syntax configuration is active
    pub syntax_highlight: Option<bool>,
}

impl TransformCache {

    pub fn new() -> Result<Self> {
        Ok(Self {
            words_re: Regex::new(r"\b\w\b")?,
            text: None,
            syntax_highlight: None,
        })
    }

    pub fn use_text_extraction(&self) -> bool {
        self.text.is_some()
    }

    pub fn use_syntax_highlight(&self) -> bool {
        self.syntax_highlight.is_some() && self.syntax_highlight.unwrap()
    }

    pub fn is_active(&self) -> bool {
        self.use_text_extraction()
            || self.use_syntax_highlight()
    }
}
