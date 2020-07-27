use std::collections::HashMap;

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
use syntect::html::highlighted_html_for_string;
use syntect::html::css_for_theme;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // NOTE: Cannot pass the RewritingError transparently as it is 
    // NOTE: not safe to Send via threads.
    #[error("{0}")]
    Rewriting(String),
}

type Result<T> = std::result::Result<T, Error>;

static HEADINGS: &str = "h1:not([id]), h2:not([id]), h3:not([id]), h4:not([id]), h5:not([id]), h6:not([id])";
static CODE: &str = "pre > code[class]";

fn scan(
    val: &str,
    strip_comments: bool,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>) -> std::result::Result<String, RewritingError> {

    let mut text_buf = String::new();
    let mut code_buf = String::new();

    let mut document_content_handlers = vec![];
    let remove_all_comments = doc_comments!(|c| {
        c.remove();
        Ok(())
    });

    if strip_comments {
        document_content_handlers.push(remove_all_comments);
    }

    rewrite_str(
        val, 
        RewriteStrSettings {
            document_content_handlers, 
            element_content_handlers: vec![
                text!(HEADINGS, |t| {
                    text_buf += t.as_str();
                    if t.last_in_text_node() {
                        headings.push(text_buf.clone());
                        text_buf.clear();
                    }
                    Ok(())
                }),
                text!(CODE, |t| {
                    code_buf += t.as_str();
                    if t.last_in_text_node() {
                        code_blocks.push(code_buf.clone());
                        code_buf.clear();
                    }
                    Ok(())
                }),
            ],
            ..Default::default()
        }
    )
}

fn transform(
    val: &str,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>,
    ps: &SyntaxSet,
    ts: &ThemeSet,
    ll: &HashMap<&str, &str>) -> std::result::Result<String, RewritingError> {

    let mut seen_headings: HashMap<String, usize> = HashMap::new();

    let lang_re = Regex::new(r"language-([^\s]+)\s?").unwrap();

    rewrite_str(
        val, 
        RewriteStrSettings {
            element_content_handlers: vec![
                element!(HEADINGS, |el| {
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
                }),
                element!(CODE, |el| {
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

                                let mut html_generator = ClassedHTMLGenerator::new(syntax, ps);
                                for line in value.lines() {
                                    html_generator.parse_html_for_line(&line);
                                }

                                let highlighted = html_generator.finalize();

                                println!("{}", highlighted);

                                let new_class_name = format!("{} code", class_name);
                                el.set_attribute("class", &new_class_name)?;

                                el.set_inner_content(&highlighted, ContentType::Html);
                                //el.append("</code>", ContentType::Html);

                                //println!("Got lang id {}", lang_id);
                                //println!("Got highlighted {}", highlighted);
                            }
                        }
                    }

                    Ok(())
                }),
            ],
            ..Default::default()
        }
    )
}

pub fn apply(doc: &str) -> Result<String> {

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let mut lang_lookup: HashMap<&str, &str> = HashMap::new();
    lang_lookup.insert("rust", "rs");

    let mut headings: Vec<String> = Vec::new();
    let mut code_blocks: Vec<String> = Vec::new();

    //let doc = "
        //<!-- Some comment to be stripped -->
        //<h1>Some heading 1 text</h1>
        //<h1>Some heading 1 text</h1>
        //<h2 id=\"foo\">Some heading 2 text with id</h2>
        //<code>no language</code>
        //<code class=\"bar\">no bar language</code>
        //<pre><code class=\"language-rust\">fn main() {}</code></pre>
//";

    //println!("{:#?}", ps.syntaxes());
    //
    //for syn in ps.syntaxes() {
        //println!("{} {:?}", syn.name, syn.file_extensions);
    //}

    match scan(doc, true, &mut headings, &mut code_blocks) {
        Ok(value) => {
            match transform(&value, &mut headings, &mut code_blocks, &ps, &ts, &lang_lookup) {
                Ok(result) => {
                    println!("Result {}", &result);
                    Ok(result)
                }
                Err(e) => Err(Error::Rewriting(e.to_string()))
            }

        }
        Err(e) => Err(Error::Rewriting(e.to_string()))
    }
}

/*
pub fn textify(src: &str) -> Result<String> {
    Ok(rewrite_str(src,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("title", |el| {
                    el.remove_and_keep_content();
                    Ok(())
                }),
                element!("body *", |el| {
                    el.remove_and_keep_content();
                    Ok(())
                }),
                //text!("body *", |t| {
                    //if t.last_in_text_node() {
                        ////t.after(" world", ContentType::Text);
                    //}
                    //Ok(())
                //})
            ],
            ..RewriteStrSettings::default()
        })?
    )
}
*/
