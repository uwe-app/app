use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use super::{loader, Error, template, utils, Options};

pub struct Parser<'a> {
    render: template::TemplateRender<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(options: &'a Options) -> Self {
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

    pub fn parse_html(&mut self, input: PathBuf) -> Result<String, Error> {
        let mut result = utils::read_string(&input).map_err(Error::from).unwrap();
        let mut data = loader::compute(&input);
        result = self
            .render
            .parse_template_string(&input, result, &mut data)?;
        result = self.render.layout(&input, result, &mut data)?;
        Ok(result)
    }

    pub fn parse_markdown(&mut self, input: PathBuf) -> Result<String, Error> {
        let content = utils::read_string(&input).map_err(Error::from).unwrap();
        let mut data = loader::compute(&input);
        let parsed = self
            .render
            .parse_template_string(&input, content, &mut data);
        match parsed {
            Ok(content) => {
                let markup = utils::render_markdown_string(&content);
                return self.render.layout(&input, markup, &mut data);
            }
            Err(e) => return Err(e),
        }
    }
}
