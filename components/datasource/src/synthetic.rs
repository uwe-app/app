use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::prelude::*;
use jsonfeed::{Feed, Item, VERSION};
use serde_json::json;

use collator::{to_href, Collate, CollateInfo};
use config::feed::{ChannelConfig, FeedConfig};
use config::{
    Config, FileInfo, FileOptions, Page, PageLink, PaginateInfo, RuntimeOptions,
};

use locale::Locales;

use crate::{DataSourceMap, Error, QueryCache, Result};

// Helper to inject synthetic pages.
fn create_synthetic(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    source: PathBuf,
    template: PathBuf,
    page_info: Arc<RwLock<Page>>,
    rewrite_index: bool,
) -> Result<()> {
    let mut file_info = FileInfo::new(options, &source, true);

    let file_opts = FileOptions {
        rewrite_index,
        base_href: &options.settings.base_href,
        ..Default::default()
    };

    let mut writer = page_info.write().unwrap();
    let dest = file_info.destination(&file_opts)?;
    writer.seal(&dest, config, options, &file_info, Some(template))?;
    drop(writer);

    // Configure a link for the synthetic page
    let href = to_href(&source, options, rewrite_index, None)?;
    let key = Arc::new(source);

    info.link(Arc::clone(&key), Arc::new(href))?;
    info.add_page(&key, dest, page_info);

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
    info.add_file(Arc::new(source), target, href, config, options)?;
    Ok(())
}

fn build_feed(
    name: &str,
    locales: &Locales,
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    feed_cfg: &FeedConfig,
    channel_cfg: &ChannelConfig,
) -> Result<Feed> {
    let url_path = if locales.languages.multi {
        Some(info.get_lang())
    } else {
        None
    };

    let base_url = options.get_canonical_url(config, url_path)?;

    let mut feed: Feed = Default::default();
    feed.version = VERSION.to_string();
    feed.language = Some(info.lang.clone());
    feed.home_page_url = Some(base_url.to_string());

    if let Some(ref authors) = config.authors {
        let authors = authors.values().map(|v| v.clone()).collect::<Vec<_>>();
        feed.authors = Some(authors);
    }

    if let Some(ref title) = channel_cfg.title {
        feed.title = title.to_string();
    }
    if let Some(ref description) = channel_cfg.description {
        feed.description = Some(description.to_string());
    }
    if let Some(ref favicon) = channel_cfg.favicon {
        feed.favicon = Some(base_url.join(favicon)?.to_string());
    }
    if let Some(ref icon) = channel_cfg.icon {
        feed.icon = Some(base_url.join(icon)?.to_string());
    }

    let page_paths = info.feeds.get(name).unwrap();
    let mut pages: Vec<&Arc<RwLock<Page>>> = page_paths
        .iter()
        .map(|pth| info.resolve(pth).unwrap())
        .collect();

    pages.sort_by(|a, b| {
        let a = &*a.read().unwrap();
        let b = &*b.read().unwrap();

        let a_val: &DateTime<Utc>;
        let b_val: &DateTime<Utc>;
        if a.created.is_some() && b.created.is_some() {
            a_val = a.created.as_ref().unwrap();
            b_val = b.created.as_ref().unwrap();
        } else if a.updated.is_some() && b.updated.is_some() {
            a_val = a.updated.as_ref().unwrap();
            b_val = b.updated.as_ref().unwrap();
        } else {
            a_val = &a.file.as_ref().unwrap().modified;
            b_val = &b.file.as_ref().unwrap().modified;
        }
        // NOTE: Compare this way around for descending order
        // NOTE: if we compared `a` to `b` instead it would be
        // NOTE: ascending. This saves us from reversing the list.
        b_val.partial_cmp(a_val).unwrap()
    });

    // Limit the number of items in the feed
    pages.truncate(*feed_cfg.limit.as_ref().unwrap());

    feed.items = pages
        .iter()
        .filter(|p| {
            let p = &*p.read().unwrap();
            !p.is_draft(options)
        })
        .map(|p| {
            let p = &*p.read().unwrap();

            let mut item: Item = Default::default();
            item.id =
                base_url.join(p.href.as_ref().unwrap()).unwrap().to_string();
            item.url = if let Some(ref permalink) = p.permalink {
                Some(base_url.join(permalink).unwrap().to_string())
            } else {
                Some(item.id.to_string())
            };

            item.title = p.title.clone();
            item.summary = p.description.clone();
            if let Some(ref created) = p.created {
                item.date_published = Some(created.to_rfc3339());
            }
            item.date_modified = if let Some(ref updated) = p.updated {
                Some(updated.to_rfc3339())
            } else {
                Some(p.file.as_ref().unwrap().modified.to_rfc3339())
            };

            // Page-level authors
            item.authors = p.authors.clone();

            // Pass through tags from the `meta` taxonomies
            if let Some(ref meta) = p.meta {
                if let Some(ref tags) = meta.get(config::TAGS) {
                    item.tags = Some(tags.to_vec());
                }
            }

            if let Some(ref entry) = p.entry {
                item.language = entry.language.clone();
                item.external_url = entry.external_url.clone();
                item.image = entry.image.clone();
                item.banner_image = entry.banner_image.clone();
                item.attachments = entry.attachments.clone();
            }

            // TODO: content

            item
        })
        .collect();

    Ok(feed)
}

// Create feed pages.
pub fn feed(
    locales: &Locales,
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
) -> Result<()> {
    if let Some(ref feed) = config.feed {
        for (name, channel) in feed.channels.iter() {
            let channel_href =
                channel.target.as_ref().unwrap().trim_start_matches("/");
            let channel_target = utils::url::to_path_separator(channel_href);
            let source_dir = options.source.join(&channel_target);

            // Data is the same for each feed
            let mut data_source: Page = Default::default();
            data_source.standalone = Some(true);
            data_source.feed = Some(build_feed(
                name, locales, config, options, info, feed, channel,
            )?);

            for feed_type in channel.types.iter() {
                let file_name = feed_type.get_name();
                let source = source_dir.join(&file_name);

                let template =
                    if let Some(ref tpl) = feed.templates.get(feed_type) {
                        options.source.join(tpl)
                    } else {
                        cache::get_feed_dir()?.join(&file_name)
                    };

                if !template.exists() {
                    return Err(Error::NoFeedTemplate(template));
                }

                let mut item_data = data_source.clone();

                let url_path = if locales.languages.multi {
                    Some(info.get_lang())
                } else {
                    None
                };

                // Update the feed url for this file
                let base_url = options.get_canonical_url(config, url_path)?;
                if let Some(ref mut feed) = item_data.feed.as_mut() {
                    let path = format!("{}/{}", channel_href, file_name);
                    feed.feed_url = Some(base_url.join(&path)?.to_string());
                }

                create_synthetic(
                    config,
                    options,
                    info,
                    source,
                    template,
                    Arc::new(RwLock::new(item_data)),
                    // NOTE: must be false otherwise we get a collision
                    // NOTE: on feed.xml and feed.json
                    false,
                )?;
            }
        }
    }
    Ok(())
}

// Copy search runtime files.
pub fn search(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
) -> Result<()> {
    if let Some(ref search) = config.search {
        let bundle = search.bundle.is_some() && search.bundle.unwrap();
        if bundle {
            let search_dir = cache::get_search_dir()?;

            let js_source = search_dir.join(config::SEARCH_JS);
            let wasm_source = search_dir.join(config::SEARCH_WASM);

            let js_value = search.js.as_ref().unwrap().to_string();
            let wasm_value = search.wasm.as_ref().unwrap().to_string();
            let js_path =
                utils::url::to_path_separator(js_value.trim_start_matches("/"));
            let wasm_path = utils::url::to_path_separator(
                wasm_value.trim_start_matches("/"),
            );

            let js_target = PathBuf::from(js_path);
            let wasm_target = PathBuf::from(wasm_path);

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
    for (q, p) in info.queries.clone().iter() {
        let queries = q.to_assign_vec();
        if queries.is_empty() {
            continue;
        }

        let page = info.get_page_mut(p).unwrap();
        for query in queries.iter() {
            let idx = map.query_index(query, cache)?;

            let res = idx
                .iter()
                .map(|v| v.to_value(query).unwrap())
                .collect::<Vec<_>>();

            let mut writer = page.write().unwrap();

            // TODO: error or warn on overwriting existing key
            writer.extra.insert(query.get_parameter(), json!(res));
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

                    create_synthetic(
                        config,
                        options,
                        info,
                        mock,
                        p.to_path_buf(),
                        Arc::new(RwLock::new(item_data)),
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

                create_synthetic(
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
