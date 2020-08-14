use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;

use collator::CollateInfo;
use config::{Config, FileInfo, FileOptions, Page, PageLink, PaginateInfo, RuntimeOptions};
//use config::link::{self, LinkOptions};

use crate::{DataSourceMap, Error, QueryCache, Result};

// Helper to inject synthetic pages.
fn create_synthetic(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    source: PathBuf,
    template: PathBuf,
    mut data: Page,
    rewrite_index: bool,
) -> Result<()> {
    let mut file_info = FileInfo::new(config, options, &source, true);

    let file_opts = FileOptions {
        rewrite_index,
        base_href: &options.settings.base_href,
        ..Default::default()
    };

    let dest = file_info.destination(&file_opts)?;

    data.seal(&dest, config, options, &file_info, Some(template))?;

    // Configure a link for the synthetic page
    let href = collator::href(&source, options, rewrite_index, None)?;
    let key = Arc::new(source);
    collator::link(info, Arc::clone(&key), Arc::new(href))?;

    // Inject the synthetic page
    info.targets.entry(Arc::clone(&key)).or_insert(dest);
    info.pages.entry(key).or_insert(data);

    Ok(())
}

// Helper to create synthetic files.
fn create_file(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    href: String,
    _base: &PathBuf,
    source: PathBuf,
    target: PathBuf,
) -> Result<()> {
    let key = Arc::new(source);
    collator::add_file(&key, target, href, info, config, options)?;
    Ok(())
}

// Create feed pages.
pub fn feed(config: &Config, options: &RuntimeOptions, info: &mut CollateInfo) -> Result<()> {
    if let Some(ref feed) = config.feed {
        println!("Creating feed pages");
        for (name, channel) in feed.channels.iter() {
            let page_paths = info.feeds.get(name).unwrap();
            println!("Got channel to process: {}", name);
        }
    }
    Ok(())
}

// Copy search runtime files.
pub fn search(config: &Config, options: &RuntimeOptions, info: &mut CollateInfo) -> Result<()> {
    if let Some(ref search) = config.search {
        let bundle = search.bundle.is_some() && search.bundle.unwrap();
        if bundle {
            let search_dir = cache::get_search_dir()?;

            let js_source = search_dir.join(config::SEARCH_JS);
            let wasm_source = search_dir.join(config::SEARCH_WASM);

            let js_value = search.js.as_ref().unwrap().to_string();
            let wasm_value = search.wasm.as_ref().unwrap().to_string();
            let js_path = utils::url::to_path_separator(js_value.trim_start_matches("/"));
            let wasm_path = utils::url::to_path_separator(wasm_value.trim_start_matches("/"));

            let js_target = options.target.join(js_path);
            let wasm_target = options.target.join(wasm_path);

            create_file(
                config,
                options,
                info,
                js_value,
                &search_dir,
                js_source,
                js_target,
            )?;
            create_file(
                config,
                options,
                info,
                wasm_value,
                &search_dir,
                wasm_source,
                wasm_target,
            )?;

            //println!("COPY THE SEARCH RUNTIME FILES");
            //std::process::exit(1);
        }
    }

    Ok(())
}

// Assign query results to the page data
pub fn assign(
    _config: &Config,
    _options: &RuntimeOptions,
    info: &mut CollateInfo,
    map: &DataSourceMap,
    cache: &mut QueryCache,
) -> Result<()> {
    for (q, p) in info.queries.iter() {
        let queries = q.to_assign_vec();
        if queries.is_empty() {
            continue;
        }

        let page = info.pages.get_mut(p).unwrap();
        for query in queries.iter() {
            let idx = map.query_index(query, cache)?;

            let res = idx
                .iter()
                .map(|v| v.to_value(query).unwrap())
                .collect::<Vec<_>>();

            // TODO: error or warn on overwriting existing key
            page.extra.insert(query.get_parameter(), json!(res));
        }
    }

    Ok(())
}

// Expand out each queries to generate a page for each item in the result set.
pub fn each(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    map: &DataSourceMap,
    cache: &mut QueryCache,
) -> Result<()> {
    let queries = info.queries.clone();

    for (q, p) in queries.iter() {
        let each = q.to_each_vec();
        if each.is_empty() {
            continue;
        }

        // Should have raw page data - note that we remove
        // the page as it is being used as an iterator
        let page = info.remove_page(p);

        let mut rewrite_index = options.settings.should_rewrite_index();
        // Override with rewrite-index page level setting
        if let Some(val) = page.rewrite_index {
            rewrite_index = val;
        }

        for each_query in each.iter() {
            let idx = map.query_index(each_query, cache)?;

            for doc in &idx {
                let mut item_data = page.clone();

                if let Some(ref id) = doc.id {
                    // Assign the document to the page data
                    item_data
                        .extra
                        .insert(each_query.get_parameter(), doc.to_value(each_query)?);

                    // Mock a source file to build a destination
                    // respecting the clean URL setting
                    let mut mock = p.parent().unwrap().to_path_buf();
                    mock.push(&id);
                    if let Some(ext) = p.extension() {
                        mock.set_extension(ext);
                    }

                    create_synthetic(
                        config,
                        options,
                        info,
                        mock,
                        p.to_path_buf(),
                        item_data,
                        rewrite_index,
                    )?;
                } else {
                    return Err(Error::DataSourceDocumentNoId);
                }
            }
        }
    }

    Ok(())
}

// Expand out result sets into page chunks.
pub fn pages(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    map: &DataSourceMap,
    cache: &mut QueryCache,
) -> Result<()> {
    let queries = info.queries.clone();
    for (q, p) in queries.iter() {
        let pages_query = q.to_page_vec();
        if pages_query.is_empty() {
            continue;
        }

        // Should have raw page data - note that we remove
        // the page as it is being used as an iterator
        let page = info.remove_page(p);

        let mut rewrite_index = options.settings.should_rewrite_index();
        // Override with rewrite-index page level setting
        if let Some(val) = page.rewrite_index {
            rewrite_index = val;
        }

        for page_query in pages_query.iter() {
            let idx = map.query_index(page_query, cache)?;

            let length = idx.len();
            let page_req = page_query.page.as_ref().unwrap();

            if page_req.size < 2 {
                return Err(Error::PageSizeTooSmall(page_req.size));
            }

            let mut total = idx.len() / page_req.size;
            if idx.len() % page_req.size != 0 {
                total += 1;
            }

            let mut chunks = Vec::new();
            let mut links = Vec::new();

            for (current, items) in idx.chunks(page_req.size).enumerate() {
                let first = current * page_req.size;
                let last = if first + page_req.size >= length {
                    length - 1
                } else {
                    first + page_req.size - 1
                };

                let size = last - first + 1;

                let page_name = format!("{}", current + 1);

                let mut item_data = page.clone();

                let file_ctx = item_data.file.as_ref().unwrap();
                let file_source = file_ctx.source.clone();

                let parent = file_source.parent().unwrap().to_path_buf();
                let mut stem = if let Some(stem) = file_source.file_stem() {
                    if stem == config::INDEX_STEM {
                        PathBuf::from("")
                    } else {
                        PathBuf::from(stem)
                    }
                } else {
                    PathBuf::from("")
                };

                stem = if rewrite_index {
                    stem.join(&page_name)
                } else {
                    stem.set_file_name(format!(
                        "{}{}",
                        stem.to_string_lossy().into_owned(),
                        page_name
                    ));
                    stem
                };

                if let Some(ext) = file_source.extension() {
                    stem.set_extension(ext);
                }

                let mock = parent.join(stem);

                let paginate = PaginateInfo {
                    total,
                    current,
                    length,
                    first,
                    last,
                    size,
                    links: Vec::new(),
                    prev: None,
                    next: None,
                    //span: Some(get_page_span(current, total, 2)),
                };

                let link = PageLink {
                    index: current,
                    name: page_name,
                    href: item_data.get_href(&mock, options)?,
                    preserve: false,
                };

                links.push(link);

                chunks.push((current, mock, file_source, item_data, paginate, items));
            }

            let length = chunks.len();
            let upto = 2;

            for (current, mock, file_source, mut item_data, mut paginate, items) in chunks {
                let mut page_links = links.clone();

                for (i, v) in page_links.iter_mut().enumerate() {
                    let after = current + upto;
                    let before: i32 = current as i32 - upto as i32;
                    let before_limit = std::cmp::max(before, 0) as usize;
                    let after_limit = std::cmp::min(after, length - 1);
                    if (i < current && i >= before_limit) || (i > current && i <= after_limit) {
                        v.preserve = true;
                    }
                }

                paginate.links = page_links;

                if current > 0 {
                    paginate.prev = Some(paginate.links[current - 1].clone());
                }

                if current < (links.len() - 1) {
                    paginate.next = Some(paginate.links[current + 1].clone());
                }

                item_data.paginate = Some(paginate);
                item_data
                    .extra
                    .insert(page_query.get_parameter(), json!(items));
                create_synthetic(
                    config,
                    options,
                    info,
                    mock,
                    file_source,
                    item_data,
                    rewrite_index,
                )?;
            }
        }
    }

    Ok(())
}
