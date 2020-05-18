use std::io;
use std::path::PathBuf;
use pulldown_cmark::{Parser as MarkdownParser, Options, html};

use super::fs;
use super::template;

pub struct Parser<'a> {
    loader: template::DataLoader,
    pub render: template::TemplateRender<'a>,
}

impl Parser<'_> {

    pub fn new(layout_name: String) -> Self {
        let loader = template::DataLoader::new();
        let render = template::TemplateRender::new(layout_name);
        Parser{loader, render}
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
                let mut options = Options::empty();
                options.insert(Options::ENABLE_STRIKETHROUGH);
                let parser = MarkdownParser::new_ext(&content, options);
                let mut markup = String::new();
                html::push_html(&mut markup, parser);

                return self.render.layout(&input, markup, &mut data)
            },
            Err(e) => return Err(e),
        }
    }
}
