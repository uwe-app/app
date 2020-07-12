use std::path::PathBuf;

use super::{RuntimeOptions, Config};

pub fn get_filters(options: &RuntimeOptions, config: &Config) -> Vec<PathBuf> {

    let source = &options.source;

    let mut filters: Vec<PathBuf> = Vec::new();

    let config_file = config.file.clone();

    let partials = options.get_partials_path();
    let includes = options.get_includes_path();
    let generator = options.get_data_sources_path();
    let resource = options.get_resources_path();

    let theme = config
        .get_book_theme_path(source);

    filters.push(partials);
    filters.push(includes);
    filters.push(generator);
    filters.push(resource);

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

    if let Some(locales_dir) = config.get_locales(source) {
        filters.push(locales_dir);
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

    // NOTE: layout comes from the build arguments so callers
    // NOTE: need to add this to the list of filters if necessary

    filters
}
