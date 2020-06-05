use std::convert::AsRef;
use std::path::Path;

use crate::{
    Error, 
    utils,
    BuildOptions
};

use super::matcher::FileType;
use super::template;
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

    fn parse_html<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        data: &mut Map<String, Value>) -> Result<String, Error> {

        let mut result = utils::read_string(&input).map_err(Error::from)?;
        result = self
            .render
            .parse_template_string(&input, &output, result, data)?;
        result = self.render.layout(&input, &output, result, data)?;
        Ok(result)
    }

    fn parse_markdown<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        data: &mut Map<String, Value>) -> Result<String, Error> {

        let content = utils::read_string(&input).map_err(Error::from)?;
        let parsed = self
            .render
            .parse_template_string(&input, &output, content, data);

        match parsed {
            Ok(content) => {
                let markup = utils::render_markdown_string(&content);
                return self.render.layout(&input, &output, markup, data);
            }
            Err(e) => return Err(e),
        }
    }

    pub fn parse<P: AsRef<Path>>(
        &mut self,
        input: P,
        output: P,
        file_type: &FileType,
        data: &mut Map<String, Value>) -> Result<String, Error> {

        match file_type {
            FileType::Html => {
                return self.parse_html(input, output, data)
            }
            FileType::Markdown => {
                return self.parse_markdown(input, output, data)
            },
            _ => Err(Error::new("parser got invalid file type".to_string()))
        }
    }
}
