use std::borrow::Cow;
use std::path::Path;

use config::{Page, FileType, FileInfo};

use crate::Error;

use super::context::Context;
use super::markdown::render_markdown_string;
use super::template::TemplateRender;

pub struct Parser<'a> {
    render: TemplateRender<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(context: &'a Context) -> Self {
        let render = TemplateRender::new(context);
        Parser { render }
    }

    pub fn register_templates_directory<P: AsRef<Path>>(
        &mut self,
        ext: &'static str,
        dir: P,
    ) -> Result<(), Error> {
        self.render.register_templates_directory(ext, dir)
    }

    fn parse_template(&mut self, info: &FileInfo, data: &mut Page) -> Result<String, Error> {
        let (content, _has_fm, _fm) =
            frontmatter::load(info.file, frontmatter::Config::new_html(false))?;
        let result = self
            .render
            .parse_template_string(info, content, data)?;
        return self.render.layout(result, data);
    }

    fn parse_markdown(&mut self, info: &FileInfo, data: &mut Page) -> Result<String, Error> {
        let (content, _has_fm, _fm) =
            frontmatter::load(info.file, frontmatter::Config::new_markdown(false))?;
        let mut result = Cow::from(self
            .render
            .parse_template_string(info, content, data)?);

        let parsed = render_markdown_string(&mut result, info.config);

        return self.render.layout(parsed, data);
    }

    pub fn parse(&mut self,info: &FileInfo, data: &mut Page) -> Result<String, Error> {
        match info.file_type {
            FileType::Template => self.parse_template(info, data),
            FileType::Markdown => self.parse_markdown(info, data),
            _ => Err(Error::ParserFileType),
        }
    }
}
