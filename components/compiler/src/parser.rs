use std::convert::AsRef;
use std::path::Path;

use config::{Page, FileType, FileInfo};

use crate::Error;

use super::markdown::render_markdown_string;

use super::context::Context;
use super::template;

pub struct Parser<'a> {
    render: template::TemplateRender<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(context: &'a Context) -> Self {
        let render = template::TemplateRender::new(context);
        Parser { render }
    }

    pub fn register_templates_directory<P: AsRef<Path>>(
        &mut self,
        ext: &'static str,
        dir: P,
    ) -> Result<(), Error> {
        self.render.register_templates_directory(ext, dir)
    }

    fn parse_template<I: AsRef<Path>, O: AsRef<Path>>(
        &mut self,
        input: I,
        output: O,
        data: &mut Page,
    ) -> Result<String, Error> {

        let (content, _has_fm, _fm) =
            frontmatter::load(&input, frontmatter::Config::new_html(false))?;
        let result = self
            .render
            .parse_template_string(&input, &output, content, data)?;
        return self.render.layout(result, data);
    }

    fn parse_markdown<I: AsRef<Path>, O: AsRef<Path>>(
        &mut self,
        input: I,
        output: O,
        data: &mut Page,
    ) -> Result<String, Error> {

        let (content, _has_fm, _fm) =
            frontmatter::load(&input, frontmatter::Config::new_markdown(false))?;
        let mut result = self
            .render
            .parse_template_string(&input, &output, content, data)?;
        result = render_markdown_string(&result);
        return self.render.layout(result, data);
    }

    pub fn parse<O: AsRef<Path>>(
        &mut self,
        info: &mut FileInfo,
        output: O,
        data: &mut Page,
    ) -> Result<String, Error> {
        match info.file_type {
            FileType::Template => self.parse_template(info.file, output, data),
            FileType::Markdown => self.parse_markdown(info.file, output, data),
            _ => Err(Error::ParserFileType),
        }
    }
}
