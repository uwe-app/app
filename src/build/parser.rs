use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use crate::{
    Error, 
    utils,
    BuildOptions
};

use super::matcher::FileType;
use super::template;

use serde_json::{Map, Value};

pub struct Parser<'a> {
    render: template::TemplateRender<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(options: &'a BuildOptions) -> Self {
        let render = template::TemplateRender::new(options);
        Parser{render}
    }

    pub fn register_templates_directory<P: AsRef<Path>>(
        &mut self,
        ext: &'static str,
        dir: P,
    ) -> Result<(), Error> {
        self.render.register_templates_directory(ext, dir)
    }

    fn parse_html(&mut self, input: PathBuf, data: &mut Map<String, Value>) -> Result<String, Error> {
        let mut result = utils::read_string(&input).map_err(Error::from).unwrap();

        result = self
            .render
            .parse_template_string(&input, result, data)?;
        result = self.render.layout(&input, result, data)?;
        Ok(result)
    }

    fn parse_markdown(&mut self, input: PathBuf, data: &mut Map<String, Value>) -> Result<String, Error> {
        let content = utils::read_string(&input).map_err(Error::from).unwrap();

        let parsed = self
            .render
            .parse_template_string(&input, content, data);
        match parsed {
            Ok(content) => {
                let markup = utils::render_markdown_string(&content);
                return self.render.layout(&input, markup, data);
            }
            Err(e) => return Err(e),
        }
    }

    pub fn parse(&mut self, input: PathBuf, file_type: FileType, data: &mut Map<String, Value>) -> Result<String, Error> {
        match file_type {
            FileType::Html => {
                return self.parse_html(input, data)
            }
            FileType::Markdown => {
                return self.parse_markdown(input, data)
            },
            _ => Err(Error::new("parser got invalid file type".to_string()))
        }
    }
}
