use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use serde_json::{json, Value};

use collator::{create_page, CollateInfo};
use config::{
    Config, IndexQuery, Page, PageLink, PaginateInfo, RuntimeOptions,
};

use crate::{CollectionsMap, Error, QueryCache, Result};

// Assign query results to the page data
pub fn assign(
    _config: &Config,
    _options: &RuntimeOptions,
    info: &mut CollateInfo,
    map: &CollectionsMap,
    cache: &mut QueryCache,
) -> Result<()> {
    // Find pages that have associated queries.
    let mut query_cache: Vec<(Vec<IndexQuery>, Arc<RwLock<Page>>)> = Vec::new();
    for (q, p) in info.queries.iter() {
        let queries = q.to_assign_vec();
        if queries.is_empty() {
            continue;
        }
        let page = info.get_page(p).unwrap();
        query_cache.push((queries, Arc::clone(page)));
    }

    // Assign collections query data to each page.
    for (queries, page) in query_cache.into_iter() {
        for query in queries.iter() {
            let mut writer = page.write().unwrap();
            assign_page_query(&mut writer, query, map, cache)?;
        }
    }

    Ok(())
}

pub fn assign_page_lookup<'a>(
    info: &CollateInfo,
    map: &CollectionsMap,
    cache: &mut QueryCache,
    needle: &Arc<PathBuf>,
    writer: &mut RwLockWriteGuard<'a, Page>,
) -> Result<()> {
    if let Some((q, _)) = info.queries.iter().find(|(_, p)| p == needle) {
        let queries = q.to_assign_vec();
        for query in queries.iter() {
            assign_page_query(writer, query, map, cache)?;
        }
    }

    Ok(())
}

// Assign collections query data for a single page.
pub fn assign_page_query<'a>(
    writer: &mut RwLockWriteGuard<'a, Page>,
    query: &IndexQuery,
    map: &CollectionsMap,
    cache: &mut QueryCache,
) -> Result<()> {
    let idx = map.query_index(query, cache)?;

    let res = idx
        .iter()
        .map(|v| v.to_value(query).unwrap())
        .collect::<Vec<_>>();

    //println!("Assigning result using key {}", query.get_parameter());
    //println!("Got result {:?}", res);

    // TODO: error or warn on overwriting existing key
    writer
        .extra
        .insert(query.get_parameter(), Value::Array(res));
    Ok(())
}

// Expand out each queries to generate a page for each item in the result set.
pub fn each(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    map: &CollectionsMap,
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
        let page = info.remove_page(p).unwrap();
        let page = page.write().unwrap();

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
                    item_data.extra.insert(
                        each_query.get_parameter(),
                        doc.to_value(each_query)?,
                    );

                    // Mock a source file to build a destination
                    // respecting the clean URL setting
                    let mut mock = p.parent().unwrap().to_path_buf();
                    mock.push(&id);
                    if let Some(ext) = p.extension() {
                        mock.set_extension(ext);
                    }

                    create_page(
                        config,
                        options,
                        info,
                        mock,
                        p.to_path_buf(),
                        Arc::new(RwLock::new(item_data)),
                        rewrite_index,
                    )?;
                } else {
                    return Err(Error::CollectionDocumentNoId);
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
    map: &CollectionsMap,
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
        let page = info.remove_page(p).unwrap();
        let page = page.read().unwrap();

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

                let item_data = page.clone();

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
                    name: (current + 1).to_string(),
                    links: Vec::new(),
                    prev: None,
                    next: None,
                    //span: Some(get_page_span(current, total, 2)),
                };

                let link = PageLink {
                    index: current,
                    name: page_name,
                    href: options.absolute(&mock, Default::default())?,
                    preserve: false,
                };

                links.push(link);

                chunks.push((
                    current,
                    mock,
                    file_source,
                    item_data,
                    paginate,
                    items,
                ));
            }

            let length = chunks.len();
            let upto = 2;

            for (
                current,
                mock,
                file_source,
                mut item_data,
                mut paginate,
                items,
            ) in chunks
            {
                let mut page_links = links.clone();

                for (i, v) in page_links.iter_mut().enumerate() {
                    let after = current + upto;
                    let before: i32 = current as i32 - upto as i32;
                    let before_limit = std::cmp::max(before, 0) as usize;
                    let after_limit = std::cmp::min(after, length - 1);
                    if (i < current && i >= before_limit)
                        || (i > current && i <= after_limit)
                    {
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

                create_page(
                    config,
                    options,
                    info,
                    mock,
                    file_source,
                    Arc::new(RwLock::new(item_data)),
                    rewrite_index,
                )?;
            }
        }
    }

    Ok(())
}
