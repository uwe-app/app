use std::sync::Arc;
use std::path::PathBuf;

use serde_json::json;

use collator::CollateInfo;
use config::{Config, RuntimeOptions, Page, FileInfo, FileOptions, PaginateInfo};

use crate::{Error, Result, DataSourceMap, QueryCache};

// Helper to inject synthetic pages.
fn create_synthetic(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    source: PathBuf,
    template: PathBuf,
    mut data: Page,
    rewrite_index: bool) -> Result<()> {

    let mut file_info = FileInfo::new(
        config,
        options,
        &source,
        true,
    );

    let file_opts = FileOptions {
        rewrite_index,
        base_href: &options.settings.base_href,
        ..Default::default()
    };

    let dest = file_info.destination(&file_opts)?;

    data.seal(
        &dest,
        config,
        options,
        &file_info,
        Some(template))?;

    // Configure a link for the synthetic page
    let href = collator::href(&source, options, rewrite_index, None)?;
    let key = Arc::new(source);
    collator::link(info, Arc::clone(&key), Arc::new(href))?;

    // Inject the synthetic page
    info.targets.entry(Arc::clone(&key)).or_insert(dest);
    info.pages.entry(key).or_insert(data);

    Ok(()) 
}

// Assign query results to the page data
pub fn assign(
    _config: &Config,
    _options: &RuntimeOptions,
    info: &mut CollateInfo,
    map: &DataSourceMap,
    cache: &mut QueryCache) -> Result<()> {

    for (q, p) in info.queries.iter() {

        let queries = q.to_assign_vec();
        if queries.is_empty() { continue; }

        let page = info.pages.get_mut(p).unwrap();
        for query in queries.iter() {
            let idx = map.query_index(query, cache)?;
            // TODO: error or warn on overwriting existing key
            page.extra.insert(query.get_parameter(), json!(idx));
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
    cache: &mut QueryCache) -> Result<()> {

    let queries = info.queries.clone();
    for (q, p) in queries.iter() {
        let pages_query = q.to_page_vec();
        if pages_query.is_empty() { continue; }

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
                    stem.join(page_name)
                } else {
                    stem.set_file_name(
                        format!("{}{}", stem.to_string_lossy().into_owned(), page_name));
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
                };

                links.push(item_data.get_href(&mock, options)?);

                chunks.push((mock, file_source, item_data, paginate, items));
            }

            for (mock, file_source, mut item_data, mut paginate, items) in chunks {
                paginate.links = links.clone();
                item_data.paginate = Some(paginate);
                item_data.extra.insert(page_query.get_parameter(), json!(items));
                create_synthetic(
                    config,
                    options,
                    info,
                    mock,
                    file_source,
                    item_data,
                    rewrite_index)?;

            }
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
    cache: &mut QueryCache) -> Result<()> {

    let queries = info.queries.clone();

    for (q, p) in queries.iter() {
        let each = q.to_each_vec();
        if each.is_empty() { continue; }

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

                if let Some(id) = doc.get("id") {
                    if let Some(id) = id.as_str() {
                        if doc.is_object() {
                            let map = doc.as_object().unwrap();
                            for (k, v) in map {
                                item_data.extra.insert(k.clone(), json!(v));
                            }
                        } else {
                            return Err(Error::DataSourceDocumentNotAnObject);
                        }

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
                            rewrite_index)?;
                    }
                } else {
                    return Err(Error::DataSourceDocumentNoId);
                }
            }
        }
    }

    Ok(())
}
