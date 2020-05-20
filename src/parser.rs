use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use handlebars::TemplateFileError;

use super::{loader,Options,template,utils};

pub struct Parser<'a> {
    loader: loader::DataLoader<'a>,
    render: template::TemplateRender<'a>,
}

impl<'a> Parser<'a> {

    pub fn new(options: &'a Options) -> Self {
        let loader = loader::DataLoader::new(options);
        let render = template::TemplateRender::new(options);
        Parser{loader, render}
    }

    pub fn register_templates_directory<P: AsRef<Path>>(&mut self, ext: &'static str, dir: P) 
        -> Result<(), TemplateFileError> {
        self.render.register_templates_directory(ext, dir)
    }

    pub fn parse_html(&mut self, input: PathBuf) -> io::Result<String> {
        let mut result = utils::read_string(&input)?;

        let mut data = loader::DataLoader::create();
        self.loader.load(&input, &mut data);

        result = self.render.parse_template_string(&input, result, &mut data)?;
        result = self.render.layout(&input, result, &mut data)?;
        Ok(result)
    }    

    pub fn parse_markdown(&mut self, input: PathBuf) -> io::Result<String> {
        let content = utils::read_string(&input)?;

        let mut data = loader::DataLoader::create();
        self.loader.load(&input, &mut data);

        let parsed = self.render.parse_template_string(&input, content, &mut data);
        match parsed {
            Ok(content) => {
                let markup = utils::render_markdown_string(&content);
                return self.render.layout(&input, markup, &mut data)
            },
            Err(e) => return Err(e),
        }
    }
}
