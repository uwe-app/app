use std::collections::HashMap;
use std::path::PathBuf;
use std::borrow::Cow;
use std::fmt::Write;
use std::sync::{Arc, RwLock};

use once_cell::sync::OnceCell;

use collator::{Collate, LinkCollate};
use compiler::{parser::Parser, BuildContext, render_markdown};
use config::{Page, CollatedPage, MenuReference};

use crate::{Error, Result};

pub fn cache() -> &'static RwLock<HashMap<MenuReference, &'static str>> {
    static INSTANCE: OnceCell<RwLock<HashMap<MenuReference, &'static str>>> = OnceCell::new();
    INSTANCE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn write<W: Write>(f: &mut W, s: &str) -> Result<()> {
    f.write_str(s).map_err(Error::from)
}

fn start_list<W: Write>(f: &mut W, name: &str) -> Result<()> {
    write(f, &format!("<ul class=\"{}\">", utils::entity::escape(name)))
}

fn pages_list<W: Write>(f: &mut W, pages: &Vec<(&String, &Arc<RwLock<Page>>)>) -> Result<()> {
    for (href, page) in pages {
        let reader = page.read().unwrap();
        write(f, "<li>")?;
        if let Some(ref title) = reader.title {

            let link_title = utils::entity::escape(title);

            // NOTE: we pass the `href` through via the `link` helper so 
            // NOTE: that links will be resolved relative to the page
            // NOTE: embedding the menu
            write(f, &format!(
                "<a href=\"{{{{ link \"{}\" }}}}\" title=\"{}\">{}</a>",
                href, link_title, link_title
            ))?;

        }
        write(f, "</li>")?;
    }

    Ok(())
}

fn end_list<W: Write>(f: &mut W) -> Result<()> {
    write(f, "</ul>")
}

/// Build a single menu reference.
fn build(
    menu: &MenuReference,
    context: &BuildContext,
    parser: &Box<dyn Parser + Send + Sync>) -> Result<String> {

    let collation = context.collation.read().unwrap();
    let lang: &str = collation.get_lang();

    let markdown = context.options.settings.types
        .as_ref().unwrap().markdown();

    let mut buf: String = String::new();

    match menu {
        MenuReference::File { ref file, .. } => {
            let file = context.options.resolve_source(file);
            let page = Page::new(&context.config, &context.options, &file)?;
            let data = CollatedPage::new(&context.config, &page, lang);
            buf = parser.parse(&file, data, true)?;

            // Check if we need to transform from markdown
            if let Some(ext) = file.extension() {
                let ext = ext.to_string_lossy().into_owned();
                if markdown.contains(&ext) {
                    buf = render_markdown(&mut Cow::from(&buf), &context.config);
                }
            }

        }
        MenuReference::Pages { ref pages, ref name } => {

            // Resolve page references to the underlying page data
            let mut page_data: Vec<(&String, &Arc<RwLock<Page>>)> = Vec::new();
            pages.iter().try_fold(&mut page_data, |acc, page_href| {
                let page_path = collation.get_link(
                    &collation.normalize(page_href));

                if let Some(ref page_path) = page_path {
                    if let Some(page) = collation.resolve(&page_path) {
                        acc.push((page_href, page));
                    } else {
                        return Err(Error::NoMenuItemPage(page_path.to_path_buf())) 
                    }
                } else {
                    return Err(Error::NoMenuItem(page_href.to_string())) 
                }

                Ok::<_, Error>(acc)
            })?;

            start_list(&mut buf, name)?;
            pages_list(&mut buf, &page_data)?;
            end_list(&mut buf)?;

            //println!("{}", buf);
            //std::process::exit(1);
        }
    }

    Ok(buf)
}

/// Compile all the menus in a collation and assign references to 
/// the compiled HTML strings to each of the pages that referenced 
/// the menu.
pub fn compile(
    context: &BuildContext,
    parser: &Box<dyn Parser + Send + Sync>) -> Result<()> {

    let collation = context.collation.read().unwrap();

    let menus: Vec<(&MenuReference, &Vec<Arc<PathBuf>>)> = collation
        .get_graph()
        .menus
        .iter()
        .collect();

    // Use a cache so we can get reference to the compiled content
    // as &'static lifetimes, see: MenuEntry
    let mut compiled = cache().write().unwrap();

    for (menu, _) in menus {
        let result = build(menu, context, parser)?;
        // Use the Box::leak trick to go to &'static strings
        compiled.insert(menu.clone(), Box::leak(Box::new(result)));
    }

    // Now we need to assign the compiled menus to each of the pages
    for (menu, paths) in collation.get_graph().menus.iter() {
        if let Some(compiled_result) = compiled.get(menu) {
            let name = match menu {
                MenuReference::File {ref name, ..} => name, 
                MenuReference::Pages{ref name, ..} => name, 
            };
            for page_path in paths.iter() {
                if let Some(page) = collation.resolve(page_path) {
                    let mut writer = page.write().unwrap();
                    if let Some(menu) = writer.menu.as_mut() {
                        if let Some (ref mut target_menu) = menu.get_mut(name) {
                            target_menu.result = compiled_result;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
