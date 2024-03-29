use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use config::{
    Config, MenuEntry, MenuReference, MenuResult, Page, RuntimeOptions,
};

use crate::{CollateInfo, Collation, Error, Result};

/// Page data stores the page path, href and corresponding data.
pub type PageData<'c> = Vec<(&'c Arc<PathBuf>, String, &'c Arc<RwLock<Page>>)>;

/// Build the HTML for a single menu and collate the list of page links
/// into a MenuResult.
pub fn build<'c>(
    options: &RuntimeOptions,
    collation: &'c CollateInfo,
    menu: &MenuEntry,
) -> Result<(MenuResult, PageData<'c>)> {
    let mut result: MenuResult = Default::default();
    let mut page_data: PageData = Vec::new();
    let mut should_sort = false;

    match menu.definition {
        MenuReference::Pages { ref pages, .. } => {
            pages.iter().try_fold(&mut page_data, |acc, page_href| {
                let page_path =
                    collation.get_link_path(&collation.normalize(page_href));

                if let Some(ref page_path) = page_path {
                    if let Some(page) = collation.resolve(&page_path) {
                        acc.push((page_path, page_href.to_string(), page));
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
        MenuReference::Directory {
            ref directory,
            ref depth,
            ref include_index,
            ..
        } => {
            should_sort = true;
            let include_index =
                include_index.is_some() && include_index.unwrap();

            let all_pages = collation.get_pages();

            let dir = utils::url::to_path_separator(
                directory.as_str().trim_start_matches("/"),
            );
            let dir_buf = options.source.join(dir);
            let dir_count = dir_buf.components().count();

            let max_depth = if let Some(depth) = depth {
                depth.clone()
            } else {
                1
            };

            let target_depth = dir_count + max_depth;

            all_pages
                .iter()
                .filter(|(k, v)| {
                    // Not inside the target directory
                    if !k.starts_with(&dir_buf) {
                        return false;
                    }

                    // Explicitly excluded from being listed using page data flag
                    let reader = v.read().unwrap();
                    if !reader.is_listable() {
                        return false;
                    }

                    let key_count = k.components().count();
                    let current_depth = key_count - dir_count;

                    if !include_index {
                        if let Some(stem) = k.file_stem() {
                            if stem == config::INDEX_STEM && current_depth == 1
                            {
                                return false;
                            }
                        }
                    }

                    if max_depth == 0 {
                        return k.starts_with(&dir_buf);
                    }

                    if key_count == target_depth + 1 {
                        if let Some(stem) = k.file_stem() {
                            k.starts_with(&dir_buf)
                                && stem == config::INDEX_STEM
                        } else {
                            false
                        }
                    } else {
                        k.starts_with(&dir_buf) && key_count <= target_depth
                    }
                })
                .try_fold(&mut page_data, |acc, (path, page)| {
                    let reader = page.read().unwrap();
                    let href = reader.href.as_ref().unwrap();
                    acc.push((path, href.clone(), page));
                    Ok::<_, Error>(acc)
                })?;
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

    // Compile page data to HTML templates when necessary.
    match menu.definition {
        MenuReference::Pages { description, .. }
        | MenuReference::Directory { description, .. } => {
            let mut buf = &mut result.value;
            template::start_list(&mut buf, &menu.name)?;
            template::pages_list(
                &mut buf,
                &page_data,
                description.is_some() && description.unwrap(),
            )?;
            template::end_list(&mut buf)?;
        }
    }

    // Assign list of pages referenced by the menu to the compiled
    // menu result so that helpers can easily find referenced pages
    result.pages = page_data
        .iter()
        .map(|(_, href, _)| Arc::new(href.clone()))
        .collect();

    Ok((result, page_data))
}

/// Compile all the menu definitions into string templates which can
/// be registered with a template parser.
pub fn compile<'c>(
    config: &'c Config,
    options: &RuntimeOptions,
    collation: &mut CollateInfo,
) -> Result<HashMap<String, MenuResult>> {
    let mut results: HashMap<String, MenuResult> = HashMap::new();

    // Compile the menu entries into template strings
    if let Some(ref menu) = config.menu {
        for (k, v) in menu.entries.iter() {
            let (result, _page_data) = build(options, collation, v)?;
            results.insert(k.to_string(), result);
        }
    }
    Ok(results)
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

/// Generate a template from page data.
mod template {
    use crate::{Error, Result};
    use std::fmt::Write;

    use super::PageData;

    fn write<W: Write>(f: &mut W, s: &str) -> Result<()> {
        f.write_str(s).map_err(Error::from)
    }

    pub(crate) fn start_list<W: Write>(f: &mut W, name: &str) -> Result<()> {
        if name.is_empty() {
            write(f, "<ul>")
        } else {
            write(
                f,
                &format!("<ul class=\"{}\">\n", utils::entity::escape(name)),
            )
        }
    }

    pub(crate) fn pages_list<'c, W: Write>(
        f: &mut W,
        pages: &PageData<'c>,
        include_description: bool,
    ) -> Result<()> {
        for (_path, href, page) in pages {
            let reader = page.read().unwrap();

            //{{{{ match href "class=\"selected\"" }}}}

            write(
                f,
                &format!(
                    r#"<li{{{{ match "{}" " class='selected'" exact=true }}}}>"#,
                    href
                ),
            )?;
            if let Some(ref title) = reader.title {
                let link_title = utils::entity::escape(title);

                // NOTE: we pass the `href` through via the `link` helper so
                // NOTE: that links will be resolved relative to the page
                // NOTE: embedding the menu
                write(
                    f,
                    &format!(
                        r#"<a href="{{{{ link "{}" }}}}" title="{}">{}</a>"#,
                        href, link_title, link_title
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

            write(f, "</li>\n")?;
        }

        Ok(())
    }

    pub(crate) fn end_list<W: Write>(f: &mut W) -> Result<()> {
        write(f, "</ul>\n")
    }
}
