use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;

use toml::Value;
use toml::de::{Error as TomlError};
use serde_derive::Deserialize;
use inflector::Inflector;
use handlebars::Handlebars;
use pulldown_cmark::{Parser as MarkdownParser, Options, html};

use log::{info, error};

use super::fs;

const INDEX_STEM: &'static str = "index";

#[derive(Deserialize,Debug)]
struct FileProperties {
    title: Option<String>,
}

pub struct Parser;

impl Parser {

    pub fn new() -> Self {
        Parser{}
    }

    // Convert a file name to title case
    fn file_auto_title(&self, input: &PathBuf) -> Option<String> {
        if let Some(nm) = input.file_stem() {
            // If the file is an index file, try to get the name 
            // from a parent directory
            if nm == INDEX_STEM {
                if let Some(p) = input.parent() {
                    return self.file_auto_title(&p.to_path_buf());
                }
            } else {
                let auto = nm.to_str().unwrap().to_string();
                let capitalized = auto.to_title_case();
                return Some(capitalized)
            }

        }
        None
    }

    fn auto_title(&self, input: &PathBuf, data: &mut BTreeMap<&str, Value>) {
        if let Some(auto) = self.file_auto_title(&input) {
            data.insert("title", Value::String(auto));
        }
    }

    //fn read_string(&self, input: &PathBuf) -> io::Result<String> {
        //let file = File::open(input)?;
        //let mut reader = BufReader::new(file);
        //let mut contents = String::new();
        //reader.read_to_string(&mut contents)?;
        //Ok(contents) 
    //}

    fn parse_template(
        &mut self,
        input: &PathBuf,
        content: String,
        data: &mut BTreeMap<&str, Value>) -> io::Result<String> {

        let mut handlebars = Handlebars::new();
        let name = &input.to_str().unwrap();
        if handlebars.register_template_string(name, &content).is_ok() {

            let filepath = input.to_str().unwrap().to_string();
            data.insert("filepath", Value::String(filepath));

            let parsed = handlebars.render(name, data);
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

    fn resolve_template(&self, input: &PathBuf) -> Option<PathBuf> {
        let name = "hypertext.hbs";
        if let Some(p) = input.parent() {
            // Note that ancestors() does not implement DoubleEndedIterator
            // so we cannot call rev()
            let mut ancestors = p.ancestors().collect::<Vec<_>>();
            ancestors.reverse();
            for p in ancestors {
                let mut copy = p.to_path_buf().clone();
                copy.push(name);
                if copy.exists() {
                    return Some(copy)
                }
            }
        }
        None
    }

    fn master(
        &mut self,
        input: &PathBuf,
        result: String, data:
        &mut BTreeMap<&str, Value>) -> io::Result<String> {
        if let Some(template) = self.resolve_template(&input) {
            // Read the master template
            let template_content = fs::read_string(&template)?;
            // Inject the result into the master template data
            // re-using the same data object
            data.insert("content", Value::String(result));
            return self.parse_template(&template, template_content, data)
        }
        Ok(result)
    }

    fn load_file_properties(&self, input: &PathBuf, data: &mut BTreeMap<&str, Value>) {
        let mut props = input.clone(); 
        props.set_extension("toml");
        if props.exists() {
            info!("TOML {}", props.display());
            let properties = fs::read_string(&props);
            match properties {
                Ok(s) => {
                    //println!("{}", s);
                    let config: Result<FileProperties, TomlError> = toml::from_str(&s);
                    match config {
                        Ok(props) => {
                            //println!("{:?}", deser);
                            if let Some(title) = props.title {
                                data.insert("title", Value::String(title));
                            }
                        },
                        Err(e) => {
                            println!("got toml parser error");
                            error!("{}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("{}", e);
                },
            }
        }
    }

    fn load_file_data(&self, input: &PathBuf, data: &mut BTreeMap<&str, Value>) {
        self.auto_title(&input, data);
        self.load_file_properties(&input, data);
    }

    pub fn parse_html(&mut self, input: PathBuf) -> io::Result<String> {
        let mut result = fs::read_string(&input)?;
        let mut data: BTreeMap<&str, Value> = BTreeMap::new();
        self.load_file_data(&input, &mut data);
        result = self.parse_template(&input, result, &mut data)?;
        result = self.master(&input, result, &mut data)?;
        Ok(result)
    }    

    pub fn parse_markdown(&mut self, input: PathBuf) -> io::Result<String> {
        let content = fs::read_string(&input)?;

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = MarkdownParser::new_ext(&content, options);

        let mut markup = String::new();
        html::push_html(&mut markup, parser);

        let mut data: BTreeMap<&str, Value> = BTreeMap::new();
        self.load_file_data(&input, &mut data);

        let mut result = self.parse_template(&input, markup, &mut data)?;
        result = self.master(&input, result, &mut data)?;

        Ok(result)
    }
}
