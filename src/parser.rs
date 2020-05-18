use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use handlebars::TemplateFileError;

use pulldown_cmark::{Parser as MarkdownParser, Options as MarkdownOptions, html};

use super::fs;
use super::template;
use super::Options;

pub struct Parser<'a> {
    options: &'a Options,
    loader: template::DataLoader,
    render: template::TemplateRender<'a>,
}

impl<'a> Parser<'a> {

    pub fn new(options: &'a Options) -> Self {
        let loader = template::DataLoader::new(options.source.clone());
        let render = template::TemplateRender::new(options.layout.clone());
        Parser{options, loader, render}
    }

    pub fn register_templates_directory<P: AsRef<Path>>(&mut self, ext: &'static str, dir: P) 
        -> Result<(), TemplateFileError> {
        self.render.register_templates_directory(ext, dir)
    }

    pub fn parse_html(&mut self, input: PathBuf) -> io::Result<String> {
        let mut result = fs::read_string(&input)?;

        let mut data = template::DataLoader::create();
        self.loader.load_file_data(&input, &mut data);

        result = self.render.parse_template_string(&input, result, &mut data)?;
        result = self.render.layout(&input, result, &mut data)?;
        Ok(result)
    }    

    pub fn parse_markdown(&mut self, input: PathBuf) -> io::Result<String> {
        let content = fs::read_string(&input)?;

        let mut data = template::DataLoader::create();
        self.loader.load_file_data(&input, &mut data);

        let parsed = self.render.parse_template_string(&input, content, &mut data);
        match parsed {
            Ok(content) => {
                let mut options = MarkdownOptions::empty();
                options.insert(MarkdownOptions::ENABLE_STRIKETHROUGH);
                let parser = MarkdownParser::new_ext(&content, options);
                let mut markup = String::new();
                html::push_html(&mut markup, parser);

                return self.render.layout(&input, markup, &mut data)
            },
            Err(e) => return Err(e),
        }
    }
}
