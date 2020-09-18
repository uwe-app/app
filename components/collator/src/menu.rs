use std::collections::HashMap;
use std::borrow::Cow;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde_json::json;
use pulldown_cmark::{Event, Tag, LinkType, CowStr};

use config::{
    MenuEntry, MenuReference, MenuResult, Page, RuntimeOptions,
    markdown,
};

use crate::{Collate, Collation, CollateInfo, Error, LinkCollate, Result};

/// Page data stores the page path, href and corresponding data.
pub type PageData<'c> = Vec<(&'c Arc<PathBuf>, String, &'c Arc<RwLock<Page>>)>;

fn write<W: Write>(f: &mut W, s: &str) -> Result<()> {
    f.write_str(s).map_err(Error::from)
}

fn start_list<W: Write>(f: &mut W, name: &str) -> Result<()> {
    write(
        f,
        &format!("<ul class=\"{}\">", utils::entity::escape(name)),
    )
}

fn pages_list<'c, W: Write>(
    f: &mut W,
    pages: &PageData<'c>,
    include_description: bool,
) -> Result<()> {
    for (path, href, page) in pages {
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

fn compile_file_menu<'c>(
    options: &RuntimeOptions,
    collation: &'c CollateInfo,
    file: &PathBuf,
    definition: &MenuReference,
) -> Result<(MenuResult, PageData<'c>)> {

    let parent = file.parent().unwrap();
    let contents = utils::fs::read_string(&file)?;

    // TODO: rewrite link references
    // TODO: transform markdown to HTML
    //
    //println!("Got menu result {}", &result.value);

    let mut result: MenuResult = Default::default();
    let mut page_data: PageData = Vec::new();

    let mut source = Cow::from(contents);
    let parser = markdown::parser(&mut source);

    let mut in_link = false;
    let mut errs: Vec<Error> = Vec::new();

    let relative = match definition {
        MenuReference::File {relative, ..} => relative.clone(),
        _ => false,
    };

    let parser = parser.map(|event| {
        match event {
            Event::Start(ref tag) => {
                match tag {
                    Tag::Link(ref kind, ref href, ref title) => {
                        in_link = true;
                        match kind {
                            LinkType::Inline => {
                                //println!("Got a link type {:?}", kind);
                                //println!("Got a link type {}", href);
                                //println!("Got a link type {}", title);

                                let target_name = utils::url::to_path_separator(
                                    href.trim_start_matches(".")
                                    .trim_start_matches("/"));

                                let target_file = parent.join(&target_name);

                                if !target_file.exists() {
                                    errs.push(
                                        Error::NoMenuLink(
                                            file.clone(),
                                            href.to_string(),
                                            target_file.clone()));
                                } else {

                                    let parent_href = match options.absolute(&parent, Default::default()) {
                                        Ok(href) => href,
                                        Err(e) => {
                                            errs.push(Error::from(e));
                                            String::new()
                                        }
                                    };

                                    match options.absolute(&target_file, Default::default()) {
                                        Ok(href) => {

                                            let file_href = collation.normalize(&href);
                                            if let Some(page_path) = collation.get_link(&file_href) {
                                                if let Some(page) = collation.resolve(page_path) {

                                                    // NOTE: that we want to use the {{ link }} template
                                                    // NOTE: call but cannot as the markdown parser
                                                    // NOTE converts the braces to HTML entities :(

                                                    //let href_template = format!("{{ link \"{}\" }}", href);
                                                    
                                                    //let href_path = page_path.to_path_buf();
                                                    //let rel_href = options.relative(&href, &href_path, &parent).unwrap();

                                                    let href = if relative {
                                                        href.trim_start_matches(&parent_href).to_string()
                                                    } else {
                                                        href
                                                    };

                                                    let event_href = CowStr::from(href.to_string());
                                                    let event_title = CowStr::from(title.to_string());

                                                    page_data.push((page_path, href, page));

                                                    return Event::Start(Tag::Link(LinkType::Inline, event_href, event_title))

                                                } else {
                                                    errs.push(Error::NoMenuPage(
                                                        file.clone(),
                                                        href.to_string(),
                                                        page_path.to_path_buf()));
                                                }
                                            } else {
                                                errs.push(
                                                    Error::NoMenuPagePath(
                                                        file.clone(), href.to_string()));
                                            }
                                        }
                                        Err(e) => errs.push(Error::from(e))
                                    }
                                }

                            }
                            _ => {}
                        }

                        event
                    }
                    _ => event
                }
            }
            Event::Text(ref _text) => {
                if in_link {
                    //println!("Got text in the link {}", text);
                }
                event
            }
            Event::End(ref tag) => {
                match tag {
                    Tag::Link(..) => {
                        in_link = false;
                        event
                    }
                    _ => event
                }
            }
            _ => event
        }
    });

    let markup = markdown::html(parser);

    // NOTE: must check errors after attempting to render 
    // NOTE: so that the parser iterator is consumed
    if !errs.is_empty() {
        let err = errs.swap_remove(0);
        return Err(err)
    }

    //println!("Got menu result {}", markup);

    result.value = markup;

    Ok((result, page_data))
}

/// Build a single menu reference.
fn build(
    menu: &Arc<MenuEntry>,
    options: &RuntimeOptions,
    collation: &CollateInfo,
) -> Result<MenuResult> {

    let mut result: MenuResult = Default::default();
    let mut page_data: PageData = Vec::new();

    match menu.definition {
        MenuReference::File { ref file, .. } => {
            let file = options.resolve_source(file);
            let (menu_result, menu_pages) = compile_file_menu(options, collation, &file, &menu.definition)?;
            result = menu_result;
            page_data = menu_pages;
        }
        MenuReference::Pages { .. } => {
            page_data = find(options, collation, &menu.definition)?;
        }
        MenuReference::Directory { .. } => {
            page_data = find(options, collation, &menu.definition)?;
        }
    }

    match menu.definition {
        MenuReference::Pages { description, .. }
        | MenuReference::Directory { description, .. } => {
            let mut buf = &mut result.value;
            start_list(&mut buf, &menu.name)?;
            pages_list(&mut buf, &page_data, description.is_some() && description.unwrap())?;
            end_list(&mut buf)?;
        }
        _ => {
            //println!("Got value {}", &result.value);
            //std::process::exit(1);
        }
    }

    Ok(result)
}

/// Find a list of pages that match a menu reference definition.
pub fn find<'c>(
    options: &RuntimeOptions,
    collation: &'c CollateInfo,
    definition: &MenuReference,
) -> Result<PageData<'c>> {

    let mut page_data: PageData = Vec::new();
    let mut should_sort = false;

    match definition {
        MenuReference::File { ref file, .. } => {
            let file = options.resolve_source(file);
            let (_, menu_pages) = compile_file_menu(options, collation, &file, definition)?;
            page_data = menu_pages;
        }
        MenuReference::Pages { ref pages, .. } => {
            pages.iter().try_fold(&mut page_data, |acc, page_href| {
                let page_path =
                    collation.get_link(&collation.normalize(page_href));

                if let Some(ref page_path) = page_path {
                    if let Some(page) = collation.resolve(&page_path) {
                        acc.push((page_path, page_href.clone(), page));
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
                .try_fold(&mut page_data, |acc, (path, page)| {
                    let reader = page.read().unwrap();
                    let href = reader.href.as_ref().unwrap();
                    acc.push((path, href.clone(), page));
                    Ok::<_, Error>(acc)
                },
            )?;
        }
    }

    if should_sort {
        // Sort by title.
        page_data.sort_by(|(_, _, a), (_, _, b)| {
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

/// Try to get the parent page for a source file path.
pub fn parent(
    options: &RuntimeOptions,
    collation: &Collation,
    file: &PathBuf,
) -> Option<Arc<RwLock<Page>>> {

    let types = options.settings.types.as_ref().unwrap();
    let render_types = types.render();

    let skip = if let Some(stem) = file.file_stem() {
        if stem == config::INDEX_STEM {
            1
        } else {
            0
        }
    } else {
        0
    };

    for p in file.ancestors().skip(skip + 1).take(1) {
        let mut parent = p.join(config::INDEX_STEM);
        for ext in render_types.iter() {
            parent.set_extension(ext);
            if let Some(ref page) = collation.resolve(&parent) {
                return Some(Arc::clone(page));
            }
        }
    }

    None
}

/// Get the pages for the components of a source file path.
pub fn components(
    options: &RuntimeOptions,
    collation: &Collation,
    file: &PathBuf,
) -> Vec<Arc<RwLock<Page>>> {
    let mut pages: Vec<Arc<RwLock<Page>>> = Vec::new();
    let types = options.settings.types.as_ref().unwrap();
    let render_types = types.render();

    let skip = if let Some(stem) = file.file_stem() {
        if stem == config::INDEX_STEM {
            1
        } else {
            0
        }
    } else {
        0
    };

    for p in file.ancestors().skip(skip) {
        if let Some(ref page) = collation.resolve(&p.to_path_buf()) {
            pages.push(Arc::clone(page));
            continue;
        }

        let mut parent = p.join(config::INDEX_STEM);

        for ext in render_types.iter() {
            parent.set_extension(ext);
            if let Some(ref page) = collation.resolve(&parent) {
                pages.push(Arc::clone(page));
            }
        }

        if p == options.source {
            break;
        }
    }

    pages
}
