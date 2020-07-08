use std::convert::AsRef;
use std::path::Path;

use config::Page;
use matcher::FileType;

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

    fn parse_template<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        data: &mut Page,
    ) -> Result<String, Error> {
        let (content, _has_fm, _fm) =
            frontmatter::load(&input, frontmatter::Config::new_html(false))?;

        let result = self
            .render
            .parse_template_string(&input, &output, content, data)?;
        return self.render.layout(&input, &output, result, data);
    }

    fn parse_markdown<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        data: &mut Page,
    ) -> Result<String, Error> {
        let (content, _has_fm, _fm) =
            frontmatter::load(&input, frontmatter::Config::new_markdown(false))?;

        let mut result = self
            .render
            .parse_template_string(&input, &output, content, data)?;
        result = render_markdown_string(&result);
        return self.render.layout(&input, &output, result, data);
    }

    pub fn parse<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        file_type: &FileType,
        data: &mut Page,
    ) -> Result<String, Error> {
        match file_type {
            FileType::Template => return self.parse_template(input, output, data),
            FileType::Markdown => return self.parse_markdown(input, output, data),
            _ => Err(Error::ParserFileType),
        }
    }
}
