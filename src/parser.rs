use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;

use toml::Value;
use handlebars::Handlebars;
use pulldown_cmark::{Parser as MarkdownParser, Options, html};

use log::{error};

use super::fs;
use super::template;

pub struct Parser<'a> {
    layout: String,
    handler: template::TemplateData,
    pub handlebars: Handlebars<'a>,
}

impl Parser<'_> {

    pub fn new(layout: String) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        let handler = template::TemplateData::new();
        Parser{layout, handlebars, handler}
    }

    fn parse_template(
        &mut self,
        input: &PathBuf,
        content: String,
        data: &mut BTreeMap<&str, Value>) -> io::Result<String> {

        let name = &input.to_str().unwrap();
        if self.handlebars.register_template_string(name, &content).is_ok() {

            let filepath = input.to_str().unwrap().to_string();
            data.insert("filepath", Value::String(filepath));

            //println!("render with name {}", name);

            let parsed = self.handlebars.render(name, data);
            match parsed {
                Ok(s) => {
                    return Ok(s)                
                },
                Err(e) => {
                    error!("{}", e);
                }
            }

        }
        Ok(content)
    }

    fn resolve_layout(&self, input: &PathBuf) -> Option<PathBuf> {
        if let Some(p) = input.parent() {
            // Note that ancestors() does not implement DoubleEndedIterator
            // so we cannot call rev()
            let mut ancestors = p.ancestors().collect::<Vec<_>>();
            ancestors.reverse();
            for p in ancestors {
                let mut copy = p.to_path_buf().clone();
                copy.push(&self.layout);
                if copy.exists() {
                    return Some(copy)
                }
            }
        }
        None
    }

    fn layout(
        &mut self,
        input: &PathBuf,
        result: String, data:
        &mut BTreeMap<&str, Value>) -> io::Result<String> {
        if let Some(template) = self.resolve_layout(&input) {
            // Read the layout template
            let template_content = fs::read_string(&template)?;
            // Inject the result into the layout template data
            // re-using the same data object
            data.insert("content", Value::String(result));
            return self.parse_template(&template, template_content, data)
        }
        Ok(result)
    }

    pub fn parse_html(&mut self, input: PathBuf) -> io::Result<String> {
        let mut result = fs::read_string(&input)?;

        let mut data = template::TemplateData::create();
        self.handler.load_file_data(&input, &mut data);

        result = self.parse_template(&input, result, &mut data)?;
        result = self.layout(&input, result, &mut data)?;
        Ok(result)
    }    

    pub fn parse_markdown(&mut self, input: PathBuf) -> io::Result<String> {
        let content = fs::read_string(&input)?;

        let mut data = template::TemplateData::create();
        self.handler.load_file_data(&input, &mut data);

        let parsed = self.parse_template(&input, content, &mut data);
        match parsed {
            Ok(content) => {
                let mut options = Options::empty();
                options.insert(Options::ENABLE_STRIKETHROUGH);
                let parser = MarkdownParser::new_ext(&content, options);
                let mut markup = String::new();
                html::push_html(&mut markup, parser);

                return self.layout(&input, markup, &mut data)
            },
            Err(e) => return Err(e),
        }
    }
}
