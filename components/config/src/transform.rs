use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TransformConfig {
    pub html: Option<HtmlTransformFlags>,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            html: Some(Default::default()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct HtmlTransformFlags {
    pub strip_comments: Option<bool>,
    pub auto_id: Option<bool>,
    pub toc: Option<bool>,

    // This flag is used internally to trigger syntax highlighting
    // transformations when the syntax configuration is active
    #[serde(skip_deserializing)]
    pub syntax_highlight: Option<bool>,
}

impl Default for HtmlTransformFlags {
    fn default() -> Self {
        Self {
            strip_comments: Some(false),
            auto_id: Some(false),
            toc: Some(false),
            syntax_highlight: Some(false),
        }
    }
}

impl HtmlTransformFlags {
    pub fn use_strip_comments(&self) -> bool {
        self.strip_comments.is_some() && self.strip_comments.unwrap()
    }

    pub fn use_auto_id(&self) -> bool {
        self.use_toc() || self.auto_id.is_some() && self.auto_id.unwrap()
    }

    pub fn use_toc(&self) -> bool {
        self.toc.is_some() && self.toc.unwrap()
    }

    pub fn use_syntax_highlight(&self) -> bool {
        self.syntax_highlight.is_some() && self.syntax_highlight.unwrap()
    }

    pub fn is_active(&self) -> bool {
        self.use_strip_comments() || self.use_toc() || self.use_auto_id() || self.use_syntax_highlight()
    }
}
