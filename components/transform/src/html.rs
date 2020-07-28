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

use config::transform::HtmlTransformFlags;

use super::{Error, Result};

static HEADINGS: &str = "h1:not([id]), h2:not([id]), h3:not([id]), h4:not([id]), h5:not([id]), h6:not([id])";
static CODE: &str = "pre > code[class]";

fn scan(
    doc: &str,
    flags: &HtmlTransformFlags,
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

    if flags.use_strip_comments() {
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

            // Must unescape text in code blocks otherwise 
            // the syntax highlighting will re-encode them
            code_buf = unescape(&code_buf);

            code_blocks.push(code_buf.clone());
            code_buf.clear();
        }
        Ok(())
    });

    if flags.use_auto_id() {
        element_content_handlers.push(auto_id_buffer);
    }

    if flags.use_syntax_highlight() {
        element_content_handlers.push(code_block_buffer); 
    }

    rewrite_str(
        doc,
        RewriteStrSettings {
            document_content_handlers, 
            element_content_handlers,
            ..Default::default()
        }
    )
}

fn rewrite(
    doc: &str,
    flags: &HtmlTransformFlags,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>) -> std::result::Result<String, RewritingError> {

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
        let value = code_blocks.remove(0);
        let class_name = el.get_attribute("class").unwrap();
        if let Some(captures) = lang_re.captures(&class_name) {
            let lang_id = captures.get(1).unwrap().as_str(); 
            if let Some(syntax) = syntax::find(lang_id) {

                let conf = syntax::conf(None);
                let highlighted = syntax::highlight(&value, syntax);
                let new_class_name = format!("{} code", class_name);
                el.set_attribute("class", &new_class_name)?;

                if conf.is_inline() {
                    el.replace(&highlighted, ContentType::Html);
                } else {
                    el.set_inner_content(&highlighted, ContentType::Html);
                }

            }
        }

        Ok(())
    });

    if flags.use_auto_id() {
        element_content_handlers.push(auto_id_rewrite);
    }

    if flags.use_syntax_highlight() {
        element_content_handlers.push(code_block_rewrite);
    }

    rewrite_str(
        doc,
        RewriteStrSettings {
            element_content_handlers,
            ..Default::default()
        }
    )
}

fn unescape(txt: &str) -> String {
    txt
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

// NOTE: This is necessary because currently the buffer text handlers
// NOTE: will not fire is there is no text (:empty) but the element 
// NOTE: handlers will fire which would cause an index out of bounds 
// NOTE: panic attempting to access the buffered data. To prevent this 
// NOTE: we strip the empty elements first.
//
// SEE: https://github.com/cloudflare/lol-html/issues/53
fn strip_empty_tags(doc: &str) -> String {
    let strip_re = Regex::new(r"(<code[^>]*></code>|<h[1-6][^>]*></h[1-6]>)").unwrap();
    strip_re.replace_all(doc, "").to_string()
}

pub fn apply(doc: &str, flags: &HtmlTransformFlags) -> Result<String> {
    let mut headings: Vec<String> = Vec::new();
    let mut code_blocks: Vec<String> = Vec::new();

    let clean = strip_empty_tags(doc);

    match scan(&clean, flags, &mut headings, &mut code_blocks) {
        Ok(value) => {
            match rewrite(&value, flags, &mut headings, &mut code_blocks) {
                Ok(result) => Ok(result),
                Err(e) => Err(Error::Rewriting(e.to_string()))
            }
        }
        Err(e) => Err(Error::Rewriting(e.to_string()))
    }
}
