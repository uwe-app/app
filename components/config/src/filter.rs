use std::path::PathBuf;

use super::{RuntimeOptions, Config};

pub fn get_filters(options: &RuntimeOptions, config: &Config) -> Vec<PathBuf> {

    let source = &options.source;

    let mut filters: Vec<PathBuf> = Vec::new();

    let config_file = config.file.clone();

    let partials = options.get_partials_path();
    let includes = options.get_includes_path();
    let data_sources = options.get_data_sources_path();
    let resource = options.get_resources_path();
    let locales = options.get_locales();

    let theme = config.get_book_theme_path(source);

    filters.push(partials);
    filters.push(includes);
    filters.push(data_sources);
    filters.push(resource);

    if let Some(_) = config.fluent {
        filters.push(locales);
    }

    if let Some(config_file) = &config_file {
        filters.push(config_file.clone());
    }

    if let Some(ref book) = config.book {
        let mut paths = book.get_paths(source);
        filters.append(&mut paths);
    }

    if let Some(ref theme) = theme {
        filters.push(theme.clone());
    }

    if let Some(ref hooks) = config.hook {
        for (_, v) in hooks {
            if let Some(source) = &v.source {
                let mut buf = source.clone();
                buf.push(source);
                filters.push(buf);
            }
        }
    }

    // Always ignore the layout
    if let Some(ref layout) = options.settings.layout {
        filters.push(layout.clone());
    }

    filters
}
