use std::collections::HashMap;

use once_cell::sync::OnceCell;

use lol_html::{
    rewrite_str,
    RewriteStrSettings,
    errors::RewritingError,
    html_content::ContentType,
    text,
    element,
    doc_comments,
};

use regex::Regex;

use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::html::ClassedHTMLGenerator;
//use syntect::html::highlighted_html_for_string;
//use syntect::html::css_for_theme;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // NOTE: Cannot pass the RewritingError transparently as it is 
    // NOTE: not safe to Send via threads.
    #[error("{0}")]
    Rewriting(String),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct TransformFlags {
    pub strip_comments: bool,
    pub auto_id: bool,
    pub syntax_highlight: bool,
}

static HEADINGS: &str = "h1:not([id]), h2:not([id]), h3:not([id]), h4:not([id]), h5:not([id]), h6:not([id])";
static CODE: &str = "pre > code[class]";

pub fn syntax() -> &'static SyntaxSet {
    static INSTANCE: OnceCell<SyntaxSet> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        SyntaxSet::load_defaults_newlines()
    })
}

pub fn themes() -> &'static ThemeSet {
    static INSTANCE: OnceCell<ThemeSet> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        ThemeSet::load_defaults()
    })
}

fn scan(
    val: &str,
    flags: &TransformFlags,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>) -> std::result::Result<String, RewritingError> {

    let mut text_buf = String::new();
    let mut code_buf = String::new();

    let mut document_content_handlers = vec![];
    let mut element_content_handlers = vec![];

    let remove_all_comments = doc_comments!(|c| {
        c.remove();
        Ok(())
    });

    if flags.strip_comments {
        document_content_handlers.push(remove_all_comments);
    }

    let auto_id_buffer = text!(HEADINGS, |t| {
        text_buf += t.as_str();
        if t.last_in_text_node() {
            headings.push(text_buf.clone());
            text_buf.clear();
        }
        Ok(())
    });

    let code_block_buffer = text!(CODE, |t| {
        code_buf += t.as_str();
        if t.last_in_text_node() {
            code_buf = code_buf.replace("&gt;", ">");
            code_buf = code_buf.replace("&lt;", "<");
            code_buf = code_buf.replace("&amp;", "&");
            code_buf = code_buf.replace("&quot;", "\"");
            code_blocks.push(code_buf.clone());
            code_buf.clear();
        }
        Ok(())
    });

    if flags.auto_id {
        element_content_handlers.push(auto_id_buffer);
    }

    if flags.syntax_highlight {
        element_content_handlers.push(code_block_buffer); 
    }

    rewrite_str(
        val, 
        RewriteStrSettings {
            document_content_handlers, 
            element_content_handlers,
            ..Default::default()
        }
    )
}

fn rewrite(
    val: &str,
    flags: &TransformFlags,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>,
    ps: &SyntaxSet,
    _ts: &ThemeSet,
    ll: &HashMap<&str, &str>) -> std::result::Result<String, RewritingError> {

    let mut seen_headings: HashMap<String, usize> = HashMap::new();
    let lang_re = Regex::new(r"language-([^\s]+)\s?").unwrap();

    let mut element_content_handlers = vec![];

    let auto_id_rewrite = element!(HEADINGS, |el| {
        if !headings.is_empty() {
            let value = headings.remove(0);
            let mut id = slug::slugify(&value);

            if seen_headings.contains_key(&id) {
                let heading_count = seen_headings.get(&id).unwrap();
                id = format!("{}-{}", &id, heading_count + 1);
            }

            el.set_attribute("id", &id)?;

            seen_headings.entry(id)
                .and_modify(|c| *c += 1)
                .or_insert(0);
        }
        Ok(())
    });

    let code_block_rewrite = element!(CODE, |el| {
        //println!("Code block trying to extract block {} {:?}", code_blocks.len(), el);

        // This is needed because empty code blocks with no text
        // will not be detected during the call to scan() as there
        // is not child text content.
        if code_blocks.is_empty() {
            return Ok(())
        }

        let value = code_blocks.remove(0);
        let class_name = el.get_attribute("class").unwrap();
        if let Some(captures) = lang_re.captures(&class_name) {
            let lang_id = captures.get(1).unwrap().as_str(); 
            if let Some(lang_ext) = ll.get(&lang_id) {
                if let Some(syntax) = ps.find_syntax_by_extension(lang_ext) {

                    //let highlighted = highlighted_html_for_string(
                        //&value,
                        //ps,
                        //syntax,
                        //&ts.themes["base16-ocean.dark"]);
                        //
                        //
                    //println!("{}", css_for_theme(&ts.themes["base16-ocean.dark"]));

                    //println!("{}", &value);

                    let mut html_generator = ClassedHTMLGenerator::new(syntax, ps);
                    for line in value.lines() {
                        html_generator.parse_html_for_line(&line);
                    }

                    let highlighted = html_generator.finalize();

                    //println!("{}", highlighted);

                    let new_class_name = format!("{} code", class_name);
                    el.set_attribute("class", &new_class_name)?;

                    el.set_inner_content(&highlighted, ContentType::Html);
                }
            }
        }

        Ok(())
    });

    if flags.auto_id {
        element_content_handlers.push(auto_id_rewrite);
    }

    if flags.syntax_highlight {
        element_content_handlers.push(code_block_rewrite);
    }

    rewrite_str(
        val, 
        RewriteStrSettings {
            element_content_handlers,
            ..Default::default()
        }
    )
}

pub fn apply(doc: &str, flags: &TransformFlags) -> Result<String> {
    let ps = syntax();
    let ts = themes();

    let mut lang_lookup: HashMap<&str, &str> = HashMap::new();
    lang_lookup.insert("rust", "rs");

    let mut headings: Vec<String> = Vec::new();
    let mut code_blocks: Vec<String> = Vec::new();

    //println!("{:#?}", ps.syntaxes());
    //
    //for syn in ps.syntaxes() {
        //println!("{} {:?}", syn.name, syn.file_extensions);
    //}

    match scan(doc, flags, &mut headings, &mut code_blocks) {
        Ok(value) => {
            match rewrite(&value, flags, &mut headings, &mut code_blocks, &ps, &ts, &lang_lookup) {
                Ok(result) => {
                    //println!("Result {}", &result);
                    Ok(result)
                }
                Err(e) => Err(Error::Rewriting(e.to_string()))
            }

        }
        Err(e) => Err(Error::Rewriting(e.to_string()))
    }
}
