use std::borrow::Cow;
use std::path::{Path, PathBuf};

use config::{Page, FileType, FileInfo};

use crate::Error;

use super::context::BuildContext;
use super::markdown::render_markdown_string;
use super::template::TemplateRender;

pub struct Parser<'a> {
    context: &'a BuildContext,
    render: TemplateRender<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(context: &'a BuildContext) -> Self {
        let render = TemplateRender::new(context);
        Parser { context, render }
    }

    pub fn register_templates_directory<P: AsRef<Path>>(
        &mut self,
        ext: &'static str,
        dir: P,
    ) -> Result<(), Error> {
        self.render.register_templates_directory(ext, dir)
    }

    fn get_front_matter_config(&mut self, file: &PathBuf) -> frontmatter::Config {
        if let Some(ext) = file.extension() {
            if ext == config::HTML {
                return frontmatter::Config::new_html(false)
            } 
        }
        frontmatter::Config::new_markdown(false)
    }

    fn parse_template(&mut self, file: &PathBuf, data: &mut Page) -> Result<String, Error> {
        let (content, _has_fm, _fm) =
            frontmatter::load(file, self.get_front_matter_config(file))?;

        let result = self.render
            .parse_template_string(file, content, data)?;
        return self.render.layout(result, data);
    }

    fn parse_markdown(&mut self, file: &PathBuf, data: &mut Page) -> Result<String, Error> {
        let (content, _has_fm, _fm) =
            frontmatter::load(file, frontmatter::Config::new_markdown(false))?;

        let mut result = Cow::from(self.render
            .parse_template_string(file, content, data)?);
        let parsed = render_markdown_string(&mut result, &self.context.config);
        return self.render.layout(parsed, data);
    }

    pub fn parse(&mut self, file: &PathBuf, data: &mut Page) -> Result<String, Error> {
        let file_type = FileInfo::get_type(file, &self.context.options.settings);
        match file_type {
            FileType::Template => self.parse_template(file, data),
            FileType::Markdown => self.parse_markdown(file, data),
            _ => Err(Error::ParserFileType),
        }
    }
}
