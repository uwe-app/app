use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde_json::json;

use config::{
    MenuEntry, MenuReference, MenuResult, MenuType, Page, RuntimeOptions,
};

use crate::{Collate, CollateInfo, Error, LinkCollate, Result};

fn write<W: Write>(f: &mut W, s: &str) -> Result<()> {
    f.write_str(s).map_err(Error::from)
}

fn start_list<W: Write>(f: &mut W, name: &str) -> Result<()> {
    write(
        f,
        &format!("<ul class=\"{}\">", utils::entity::escape(name)),
    )
}

fn pages_list<W: Write>(
    f: &mut W,
    pages: &Vec<(String, &Arc<RwLock<Page>>)>,
    include_description: bool,
) -> Result<()> {
    for (href, page) in pages {
        let reader = page.read().unwrap();
        write(f, "<li>")?;
        if let Some(ref title) = reader.title {
            let link_title = utils::entity::escape(title);

            // NOTE: we pass the `href` through via the `link` helper so
            // NOTE: that links will be resolved relative to the page
            // NOTE: embedding the menu
            write(
                f,
                &format!(
                    "<a href=\"{{{{ link {} }}}}\" title=\"{}\">{}</a>",
                    json!(href), link_title, link_title
                ),
            )?;

            if include_description {
                if let Some(ref description) = reader.description {
                    write(
                        f,
                        &format!(
                            "<p>{}</p>",
                            utils::entity::escape(description)
                        ),
                    )?;
                }
            }
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
    collation: &CollateInfo,
) -> Result<MenuResult> {
    let markdown = options.settings.types.as_ref().unwrap().markdown();

    //let all_pages = collation.get_pages();
    let mut result: MenuResult = Default::default();
    let mut buf = &mut result.value;
    let mut page_data: Vec<(String, &Arc<RwLock<Page>>)> = Vec::new();

    match menu.definition {
        MenuReference::File { ref file } => {
            let file = options.resolve_source(file);
            write(buf, &utils::fs::read_string(&file)?)?;
            // Check if we need to transform from markdown when
            // the helper renders the menu
            if let Some(ext) = file.extension() {
                let ext = ext.to_string_lossy().into_owned();
                if markdown.contains(&ext) {
                    result.kind = MenuType::Markdown;
                }
            }
        }
        MenuReference::Pages { .. } => {
            page_data = find(&menu.definition, options, collation)?;
        }
        MenuReference::Directory { .. } => {
            page_data = find(&menu.definition, options, collation)?;
        }
    }

    match menu.definition {
        MenuReference::Pages { description, .. }
        | MenuReference::Directory { description, .. } => {
            start_list(&mut buf, &menu.name)?;
            pages_list(&mut buf, &page_data, description.is_some() && description.unwrap())?;
            end_list(&mut buf)?;
        }
        _ => {}
    }

    Ok(result)
}

/// Find a list of pages that match a menu reference definition.
pub fn find<'c>(
    definition: &MenuReference,
    options: &RuntimeOptions,
    collation: &'c CollateInfo,
) -> Result<Vec<(String, &'c Arc<RwLock<Page>>)>> {

    let mut page_data: Vec<(String, &Arc<RwLock<Page>>)> = Vec::new();
    let mut should_sort = false;

    match definition {
        MenuReference::Pages { ref pages, .. } => {
            pages.iter().try_fold(&mut page_data, |acc, page_href| {
                let page_path =
                    collation.get_link(&collation.normalize(page_href));

                if let Some(ref page_path) = page_path {
                    if let Some(page) = collation.resolve(&page_path) {
                        acc.push((page_href.clone(), page));
                    } else {
                        return Err(Error::NoMenuItemPage(
                            page_path.to_path_buf(),
                        ));
                    }
                } else {
                    return Err(Error::NoMenuItem(page_href.to_string()));
                }

                Ok::<_, Error>(acc)
            })?;
        }
        MenuReference::Directory { ref directory, ref depth, .. } => {

            should_sort = true;

            let all_pages = collation.get_pages();

            let dir = utils::url::to_path_separator(
                directory.trim_start_matches("/"),
            );
            let dir_buf = options.source.join(dir);
            let dir_count = dir_buf.components().count();

            let max_depth = if let Some(depth) = depth { depth.clone() } else { 1 };
            let target_depth = dir_count + max_depth;

            all_pages
                .iter()
                .filter(|(k, _)| {

                    if max_depth == 0 {
                        return k.starts_with(&dir_buf);
                    }

                    let key_count = k.components().count();

                    if key_count == target_depth + 1 {
                        if let Some(stem) = k.file_stem() {
                            stem == config::INDEX_STEM
                        } else {
                            false
                        }
                    } else {
                        //println!("k : {}", k.display());
                        k.starts_with(&dir_buf) && key_count <= target_depth
                    }
                })
                .try_fold(&mut page_data, |acc, (_path, page)| {
                    let reader = page.read().unwrap();
                    let href = reader.href.as_ref().unwrap();
                    acc.push((href.clone(), page));
                    Ok::<_, Error>(acc)
                },
            )?;
        }
        _ => {}
    }

    if should_sort {
        // Sort by title.
        page_data.sort_by(|(_, a), (_, b)| {
            let a = &*a.read().unwrap();
            let b = &*b.read().unwrap();
            let s1 = a.title.as_ref().map(|x| &**x).unwrap_or("");
            let s2 = b.title.as_ref().map(|x| &**x).unwrap_or("");
            s1.partial_cmp(s2).unwrap()
        });
    }

    Ok(page_data)
}

/// Compile all the menus in a collation and assign references to
/// the compiled HTML strings to each of the pages that referenced
/// the menu.
pub fn compile(
    options: &RuntimeOptions,
    collation: &mut CollateInfo,
) -> Result<()> {
    let mut compiled: Vec<(Arc<MenuEntry>, MenuResult, Vec<Arc<PathBuf>>)> =
        Vec::new();

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

    Ok(())
}
