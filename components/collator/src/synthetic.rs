use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::prelude::*;
use jsonfeed::{Feed, Item, VERSION};

use config::{
    feed::{ChannelConfig, FeedConfig},
    plugin_cache::PluginCache,
    Config, Page, Plugin, RuntimeOptions,
};

use locale::Locales;

use crate::{to_href, Collate, CollateInfo, Error, Result};

// Helper to inject synthetic pages.
pub fn create_page(
    config: &Config,
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    source: PathBuf,
    template: PathBuf,
    page_info: Arc<RwLock<Page>>,
    rewrite_index: bool,
) -> Result<()> {
    let dest = options
        .destination()
        .rewrite_index(rewrite_index)
        .build(&source)?;

    let mut writer = page_info.write().unwrap();
    writer.seal(config, options, &source, &dest, Some(template))?;
    drop(writer);

    // Configure a link for the synthetic page
    let href = to_href(&source, options, rewrite_index, None)?;
    let key = Arc::new(source);

    info.link(Arc::clone(&key), Arc::new(href))?;
    info.add_page(&key, dest, page_info);

    Ok(())
}

// Helper to create synthetic files.
pub fn create_file(
    options: &RuntimeOptions,
    info: &mut CollateInfo,
    source: PathBuf,
    target: PathBuf,
    href: String,
    base: Option<&PathBuf>,
) -> Result<()> {
    info.add_file(options, Arc::new(source), target, href, base)?;
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
    let url_path = if locales.is_multi_lingual() {
        Some(info.get_lang())
    } else {
        None
    };

    let base_url = options.get_canonical_url(config, url_path)?;

    let mut feed: Feed = Default::default();
    feed.version = VERSION.to_string();
    feed.language = Some(info.lang.clone());
    feed.home_page_url = Some(base_url.to_string());

    if !config.authors().is_empty() {
        let authors = config
            .authors()
            .values()
            .cloned()
            .map(|a| a.into_json_feed())
            .collect::<Vec<_>>();

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
                Some(base_url.join(permalink.as_str()).unwrap().to_string())
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
            item.authors = if let Some(ref author_refs) = p.authors() {
                Some(
                    config
                        .authors()
                        .iter()
                        .filter(|(k, _)| author_refs.contains(k))
                        .map(|(_, v)| v)
                        .cloned()
                        .map(|a| a.into_json_feed())
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };

            // Pass through tags from the `meta` taxonomies
            if let Some(ref taxonomies) = p.taxonomies {
                if let Some(ref tags) = taxonomies.get(config::TAGS) {
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

fn find_feed_plugin<'a>(
    feed: &FeedConfig,
    plugins: Option<&'a PluginCache>,
) -> Option<&'a Plugin> {
    let plugin_name = feed.plugin.as_ref().unwrap();
    if let Some(cache) = plugins {
        // NOTE: we only look for a direct dependency at the moment
        for (_, plugin) in cache.plugins().iter() {
            if &plugin.name == plugin_name {
                return Some(plugin);
            }
        }
    }
    None
}

// Create feed pages.
pub fn feed(
    feed: &FeedConfig,
    locales: &Locales,
    config: &Config,
    options: &RuntimeOptions,
    plugins: Option<&PluginCache>,
    info: &mut CollateInfo,
) -> Result<()> {
    let plugin_name = feed.plugin.as_ref().unwrap().clone();

    let plugin = find_feed_plugin(feed, plugins)
        .ok_or_else(|| Error::NoFeedPlugin(plugin_name.clone()))?;

    let engine_templates =
        plugin.templates().get(config.engine()).ok_or_else(|| {
            Error::NoFeedPluginTemplateEngine(
                plugin_name.clone(),
                config.engine().to_string(),
            )
        })?;

    let plugin_layouts = engine_templates
        .layouts
        .as_ref()
        .ok_or_else(|| Error::NoFeedPluginLayout(plugin_name.clone()))?;

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
            let file_extension = feed_type.get_extension();
            let source = source_dir.join(&file_name);

            let template: Option<PathBuf> = if let Some(ref partial_key) =
                feed.names.get(feed_type)
            {
                let full_partial_key =
                    format!("{}.{}", partial_key, &file_extension);
                if let Some(ref partial) = plugin_layouts.get(&full_partial_key)
                {
                    Some(partial.to_path_buf(plugin.base()))
                } else {
                    None
                }
            } else {
                None
            };

            if template.is_none() {
                return Err(Error::NoFeedPartialPath(feed_type.to_string()));
            }

            let template = template.unwrap();

            if !template.exists() || !template.is_file() {
                return Err(Error::NoFeedTemplate(template));
            }

            let mut item_data = data_source.clone();

            let url_path = if locales.is_multi_lingual() {
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

            create_page(
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
    Ok(())
}
