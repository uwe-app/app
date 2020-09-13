use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt::Write;
use std::sync::{Arc, RwLock};

use collator::{CollateInfo, Collate, LinkCollate};
use config::{Config, RuntimeOptions, Page, MenuEntry, MenuReference, MenuResult, MenuType};

use crate::{Error, Result};

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
    menu: &Arc<MenuEntry>,
    options: &RuntimeOptions,
    collation: &CollateInfo) -> Result<MenuResult> {

    let markdown = options.settings.types
        .as_ref().unwrap().markdown();

    let mut result: MenuResult = Default::default();
    let mut buf = &mut result.value;

    match menu.definition {
        MenuReference::File { ref file } => {
            let file = options.resolve_source(file);
            result.value = utils::fs::read_string(&file)?;
            // Check if we need to transform from markdown when
            // the helper renders the menu
            if let Some(ext) = file.extension() {
                let ext = ext.to_string_lossy().into_owned();
                if markdown.contains(&ext) {
                    result.kind = MenuType::Markdown;
                }
            }
        }
        MenuReference::Pages { ref pages } => {
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

            start_list(&mut buf, &menu.name)?;
            pages_list(&mut buf, &page_data)?;
            end_list(&mut buf)?;
        }
    }

    Ok(result)
}

/// Compile all the menus in a collation and assign references to 
/// the compiled HTML strings to each of the pages that referenced 
/// the menu.
pub fn compile(
    options: &RuntimeOptions,
    collation: &mut CollateInfo) -> Result<()> {

    let mut compiled: Vec<(Arc<MenuEntry>, MenuResult, Vec<Arc<PathBuf>>)> = Vec::new();

    for (menu, paths) in collation.get_graph().menus.sources.iter() {
        let result = build(&menu, options, collation)?;
        compiled.push((Arc::clone(menu), result, paths.to_vec()));
    }

    let graph = collation.get_graph_mut();
    for (menu, result, paths) in compiled {
        let res = Arc::new(result);
        for path in paths {
            let map = graph.menus.mapping.entry(path).or_insert(HashMap::new());
            map.insert(menu.name.clone(), Arc::clone(&res));
        }
        graph.menus.results.entry(Arc::clone(&menu)).or_insert(res);
    }

    //println!("Menu IR {:#?}", graph.menus.mapping);
    //std::process::exit(1);

    Ok(())
}
