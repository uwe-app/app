use std::convert::TryInto;
use std::path::PathBuf;

use collator::{CollateInfo, CollateRequest, CollateResult};
use config::{Config, LocaleMap, RuntimeOptions};

use crate::{Error, Result};

fn get_locale_target(
    lang: &str,
    locales: &LocaleMap,
    base: &PathBuf,
) -> PathBuf {
    if locales.multi {
        base.join(lang)
    } else {
        base.clone()
    }
}

/// Get the default fallback collation.
pub(crate) async fn collate(
    locales: &LocaleMap,
    config: &Config,
    options: &RuntimeOptions,
) -> Result<CollateInfo> {
    let lang = &locales.fallback;
    let path = get_locale_target(lang, locales, &options.base);

    // Collate page data for later usage
    let req = CollateRequest { locales, config, options };

    let mut res = CollateResult::new(lang.to_string(), path);
    let mut errors = collator::walk(req, &mut res).await?;
    if !errors.is_empty() {
        // TODO: print all errors?
        let e = errors.swap_remove(0);
        return Err(Error::Collator(e));
    }

    let collation: CollateInfo = res.try_into()?;

    /*
    // Find and transform localized pages
    collator::localize(
        &self.context.config,
        &self.context.options,
        &self.context.options.locales,
        &mut collation,
    )
    .await?;

    */

    Ok(collation)
}

/// Take the fallback locale and extract pages for each of the alternative languages.
pub(crate) async fn extract(
    locales: &LocaleMap,
    fallback: &mut CollateInfo,
    languages: Vec<&str>,
    config: &Config,
    options: &RuntimeOptions,
) -> Result<Vec<CollateInfo>> {
    let values: Vec<CollateInfo> = languages
        .iter()
        .map(|lang| {
            let path = get_locale_target(lang, locales, &options.base);
            //println!("Collate for alternative language {:?}", lang);
            //println!("Using fallback {:?}", &fallback.lang);
            CollateInfo::new(lang.to_string(), path)
        })
        .collect();

    Ok(values)
}

// Localize logic involves another pass as we can't guarantee the order
// that pages are discovered so this allows us to ensure we have page
// data for the default fallback locale before we assign page data for
// locales specific pages. Which allows us to inherit page data from the
// fallback page.
/*
pub async fn localize(
    config: &Config,
    options: &RuntimeOptions,
    locales: &LocaleMap,
    info: &mut CollateInfo,
) -> Result<()> {
    let cache: Vec<LocalePage> = get_locale_page_cache(options, locales, info);

    let pages = info
        .pages
        .get_mut(&options.locales.fallback)
        .unwrap()
        .clone();

    for entry in cache {
        let lang = entry.locale_id.clone();
        let map = info.pages.entry(entry.locale_id).or_insert(HashMap::new());
        let mut page_info = entry.page;
        let use_fallback = page_info.fallback.is_some() && page_info.fallback.unwrap();

        // Inherit from the fallback page when it exists
        if let Some(fallback_page) = pages.get(&entry.fallback) {
            let file_context = fallback_page.file.as_ref().unwrap();
            let source = file_context.source.clone();
            // NOTE: Must clone the fallback page
            let mut fallback_page = fallback_page.clone();

            let template = if use_fallback {
                fallback_page.file.as_ref().unwrap().template.clone()
            } else {
                page_info.file.as_ref().unwrap().template.clone()
            };

            let mut tmp: Page = Default::default();

            tmp.append(&mut fallback_page);
            tmp.append(&mut page_info);

            let mut rewrite_index = options.settings.should_rewrite_index();
            // Override with rewrite-index page level setting
            if let Some(val) = tmp.rewrite_index {
                rewrite_index = val;
            }

            // Must seal() again so the file paths are correct
            let mut file_info = FileInfo::new(config, options, &source, false);
            let file_opts = FileOptions {
                rewrite_index,
                base_href: &options.settings.base_href,
                ..Default::default()
            };
            let dest = file_info.destination(&file_opts)?;
            tmp.seal(&dest, config, options, &file_info, Some(template), &lang)?;

            // Ensure we are putting the file in the correct locale specific location
            let locale_target = options.target.join(&lang);
            tmp.rewrite_target(&options.target, &locale_target)?;

            page_info = tmp;
        }
        map.insert(Arc::new(entry.fallback), page_info);

        info.remove_page(&entry.path, options);
    }

    //for (k, _v) in info.pages.iter() {
    //println!("Got page key {:?}", k);
    //}

    //println!("Locale pages {:#?}", info.locale_pages);

    Ok(())
}
*/
