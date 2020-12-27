use std::collections::HashMap;

use lol_html::{
    doc_comments, element, errors::RewritingError, html_content::ContentType,
    rewrite_str, text, RewriteStrSettings,
};

use regex::{Captures, Regex};
use htmlentity::entity;

use config::transform::HtmlTransformFlags;
use toc::TableOfContents;

use crate::cache::TransformCache;
use crate::text::TextExtraction;
use crate::{Error, Result};

static HEADINGS: &str = "h1, h2, h3, h4, h5, h6";
static CODE: &str = "pre > code[class]";
static TITLE: &str = "title";
static TEXT: &str = "p, [data-index] *";

fn scan(
    doc: &str,
    flags: &HtmlTransformFlags,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>,
    cache: &mut TransformCache,
) -> std::result::Result<String, RewritingError> {
    let mut text_buf = String::new();
    let mut code_buf = String::new();
    let mut title_buf = String::new();

    let mut document_content_handlers = vec![];
    let mut element_content_handlers = vec![];

    let extract_text = cache.text.is_some();
    let highlight = cache.use_syntax_highlight();

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
            headings.push(entity::decode(&text_buf));
            text_buf.clear();
        }
        Ok(())
    });

    let code_block_buffer = text!(CODE, |t| {
        code_buf += t.as_str();
        if t.last_in_text_node() {
            // Must unescape text in code blocks otherwise
            // the syntax highlighting will re-encode them
            code_buf = utils::entity::unescape(&code_buf);

            code_blocks.push(code_buf.clone());
            code_buf.clear();
        }
        Ok(())
    });

    let extract_text_title = text!(TITLE, |t| {
        if let Some(txt) = cache.text.as_mut() {
            // If there are multiple title tags the last
            // one will win
            title_buf += t.as_str();
            if t.last_in_text_node() {
                txt.title = Some(title_buf.clone());
                title_buf.clear();
            }
        }
        Ok(())
    });

    if flags.use_auto_id() {
        element_content_handlers.push(auto_id_buffer);
    }

    if highlight {
        element_content_handlers.push(code_block_buffer);
    }

    if extract_text {
        element_content_handlers.push(extract_text_title);
    }

    rewrite_str(
        doc,
        RewriteStrSettings {
            document_content_handlers,
            element_content_handlers,
            ..Default::default()
        },
    )
}

fn rewrite(
    doc: &str,
    flags: &HtmlTransformFlags,
    headings: &mut Vec<String>,
    code_blocks: &mut Vec<String>,
    toc: &mut Option<TableOfContents>,
    cache: &mut TransformCache,
) -> std::result::Result<String, RewritingError> {
    let mut seen_headings: HashMap<String, usize> = HashMap::new();
    let lang_re = Regex::new(r"language-([^\s]+)\s?").unwrap();

    let extract_text = cache.text.is_some();
    let use_words = flags.use_words();
    let highlight = cache.use_syntax_highlight();

    let mut text_buf = String::new();
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

            seen_headings.entry(id).and_modify(|c| *c += 1).or_insert(0);
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

    let extract_text_content = text!(TEXT, |t| {
        if let Some(txt) = cache.text.as_mut() {
            text_buf += t.as_str();
            if t.last_in_text_node() {
                if use_words {
                    let count = cache.words_re.find_iter(&text_buf).count();
                    txt.words += count;
                }
                txt.chunks.push(text_buf.clone());
                text_buf.clear();
            }
        }
        Ok(())
    });

    if flags.use_auto_id() {
        element_content_handlers.push(auto_id_rewrite);
    }

    if highlight {
        element_content_handlers.push(code_block_rewrite);
    }

    if extract_text {
        element_content_handlers.push(extract_text_content);
    }

    rewrite_str(
        doc,
        RewriteStrSettings {
            element_content_handlers,
            ..Default::default()
        },
    )
}

// NOTE: This is necessary because currently the buffer text handlers
// NOTE: will not fire if there is no text (:empty) but the element
// NOTE: handlers will fire which would cause an index out of bounds
// NOTE: panic attempting to access the buffered data. To prevent this
// NOTE: we strip the empty elements first.
//
// SEE: https://github.com/cloudflare/lol-html/issues/53
fn strip_empty_tags(doc: &str) -> String {
    let strip_re =
        Regex::new(r"(<code[^>]*></code>|<h[1-6][^>]*></h[1-6]>)").unwrap();
    strip_re.replace_all(doc, "").to_string()
}

fn toc_replace(doc: &str, toc: &TableOfContents) -> Result<String> {
    let toc_re = Regex::new(
        "<toc data-tag=\"(ol|ul)\" data-class=\"([^\"]*)\" data-from=\"(h[1-6])\" data-to=\"(h[1-6])\" />"
    ).unwrap();

    // TODO: allow multiple replacements here, callers might want a toggle side menu
    // TODO: and a static toc at the top of the page!

    if let Some(groups) = toc_re.captures(doc) {
        let tag = groups.get(1).unwrap().as_str();
        let class = groups.get(2).unwrap().as_str();
        let from = groups.get(3).unwrap().as_str();
        let to = groups.get(4).unwrap().as_str();

        let markup = toc.to_html_string(tag, class, from, to)?;
        let res = toc_re.replace(doc, markup.as_str());
        return Ok(res.to_owned().to_string());
    }

    Ok(doc.to_string())
}

fn word_replace(doc: &str, text: &TextExtraction) -> Result<String> {
    let word_re = Regex::new("<words( data-avg=\"([0-9]+)\")? />").unwrap();
    let res = word_re
        .replace_all(doc, |caps: &Captures| {
            let mut value: usize = text.words;
            let avg_attr = caps.get(2);
            if let Some(avg) = avg_attr {
                let avg: usize = match avg.as_str().parse() {
                    Ok(res) => res,
                    Err(_) => 250,
                };

                // It doesn't make sense to show zero minutes for reading
                // time so we ensure it has a minimum value. Also, we set
                // it to two so that it is always a plural value as a workaround
                // for the pluralization issue so callers can always safely use:
                //
                // {{words time=true}} minutes
                //
                value = std::cmp::max(value / avg, 2);
            }

            value.to_string()
        })
        .to_string();

    Ok(res.to_string())
}

pub fn apply(
    doc: &str,
    flags: &HtmlTransformFlags,
    cache: &mut TransformCache,
) -> Result<String> {
    let mut headings: Vec<String> = Vec::new();
    let mut code_blocks: Vec<String> = Vec::new();

    let clean = strip_empty_tags(doc);
    let value = scan(&clean, flags, &mut headings, &mut code_blocks, cache)
        .map_err(|e| Error::Rewriting(e.to_string()))?;

    let mut toc = if flags.use_toc() {
        Some(TableOfContents::new())
    } else {
        None
    };

    let mut result = rewrite(
        &value,
        flags,
        &mut headings,
        &mut code_blocks,
        &mut toc,
        cache,
    )
    .map_err(|e| Error::Rewriting(e.to_string()))?;

    if flags.use_toc() {
        result = toc_replace(&result, toc.as_ref().unwrap())?;
    }

    if flags.use_words() {
        if let Some(ref text) = cache.text {
            result = word_replace(&result, text)?;
        }
    }

    Ok(result)
}
