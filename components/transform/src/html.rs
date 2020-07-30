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
use toc::TableOfContents;

use super::{Error, Result};

static HEADINGS: &str = "h1, h2, h3, h4, h5, h6";
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
    code_blocks: &mut Vec<String>,
    toc: &mut Option<TableOfContents>) -> std::result::Result<String, RewritingError> {

    let mut seen_headings: HashMap<String, usize> = HashMap::new();
    let lang_re = Regex::new(r"language-([^\s]+)\s?").unwrap();

    let mut element_content_handlers = vec![];

    let auto_id_rewrite = element!(HEADINGS, |el| {
        if !headings.is_empty() {
            let value = headings.remove(0);
            let id_attr = el.get_attribute("id");

            let mut id = if let Some(ref val) = id_attr {
                val.to_string() 
            } else {
                slug::slugify(&value)
            };

            if seen_headings.contains_key(&id) {
                let heading_count = seen_headings.get(&id).unwrap();
                id = format!("{}-{}", &id, heading_count + 1);
            }

            if let None = id_attr {
                el.set_attribute("id", &id)?;
            }

            if let Some(toc) = toc.as_mut() {
                toc.add(&el.tag_name(), &id, &value)?;
            }

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

// Content in code blocks has already been escaped
// so we need to unescape it before highlighting.
fn unescape(txt: &str) -> String {
    txt
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
}

// NOTE: This is necessary because currently the buffer text handlers
// NOTE: will not fire if there is no text (:empty) but the element 
// NOTE: handlers will fire which would cause an index out of bounds 
// NOTE: panic attempting to access the buffered data. To prevent this 
// NOTE: we strip the empty elements first.
//
// SEE: https://github.com/cloudflare/lol-html/issues/53
fn strip_empty_tags(doc: &str) -> String {
    let strip_re = Regex::new(r"(<code[^>]*></code>|<h[1-6][^>]*></h[1-6]>)").unwrap();
    strip_re.replace_all(doc, "").to_string()
}

fn toc_replace(doc: &str, toc: &TableOfContents) -> Result<String> {
    let toc_re = Regex::new(
        "<toc data-tag=\"(ol|ul)\" data-class=\"([^\"]*)\" data-from=\"(h[1-6])\" data-to=\"(h[1-6])\" />"
    ).unwrap();

    if let Some(groups) = toc_re.captures(doc) {
        let tag = groups.get(1).unwrap().as_str(); 
        let class = groups.get(2).unwrap().as_str(); 
        let from = groups.get(3).unwrap().as_str(); 
        let to = groups.get(4).unwrap().as_str(); 

        let markup = toc.to_html_string(tag, class, from, to)?;
        let res = toc_re.replace(doc, markup.as_str());
        return Ok(res.to_owned().to_string())
    }
    Ok(doc.to_string())
}

pub fn apply(doc: &str, flags: &HtmlTransformFlags) -> Result<String> {
    let mut headings: Vec<String> = Vec::new();
    let mut code_blocks: Vec<String> = Vec::new();

    let clean = strip_empty_tags(doc);
    let value = scan(&clean, flags, &mut headings, &mut code_blocks)
        .map_err(|e| Error::Rewriting(e.to_string()))?;

    let mut toc = if flags.use_toc() { Some(TableOfContents::new()) } else { None };
    let result = rewrite(&value, flags, &mut headings, &mut code_blocks, &mut toc)
        .map_err(|e| Error::Rewriting(e.to_string()))?;

    if flags.use_toc() {
        let res = toc_replace(&result, toc.as_ref().unwrap())?;
        return Ok(res)
    }

    Ok(result)
}
