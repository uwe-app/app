use std::convert::AsRef;
use std::path::Path;

use crate::{
    Error, 
    utils
};

use super::matcher::FileType;
use super::loader;
use super::template;
use super::frontmatter;
use super::context::Context;

use serde_json::{Map, Value};

pub struct Parser<'a> {
    render: template::TemplateRender<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(context: &'a Context) -> Self {
        let render = template::TemplateRender::new(context);
        Parser{render}
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
        data: &mut Map<String, Value>) -> Result<String, Error> {

        let (content, has_fm, fm) = frontmatter::load(
            &input, frontmatter::Config::new_html(false))?;

        if has_fm {
            loader::parse_into(fm, data)?;
        }

        let result = self
            .render
            .parse_template_string(&input, &output, content, data)?;
        return self.render.layout(&input, &output, result, data);
    }

    fn parse_markdown<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        data: &mut Map<String, Value>) -> Result<String, Error> {

        let (content, has_fm, fm) = frontmatter::load(
            &input, frontmatter::Config::new_markdown(false))?;

        if has_fm {
            loader::parse_into(fm, data)?;
        }

        let mut result = self
            .render
            .parse_template_string(&input, &output, content, data)?;
        result = utils::render_markdown_string(&result);
        return self.render.layout(&input, &output, result, data);
    }

    pub fn parse<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        file_type: &FileType,
        data: &mut Map<String, Value>) -> Result<String, Error> {

        match file_type {
            FileType::Template => {
                return self.parse_template(input, output, data)
            }
            FileType::Markdown => {
                return self.parse_markdown(input, output, data)
            },
            _ => Err(Error::new("parser got invalid file type".to_string()))
        }
    }
}
