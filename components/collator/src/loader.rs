use std::path::Path;

use inflector::Inflector;
use log::warn;

use config::{Config, FileType, Page, RuntimeOptions};

use crate::{Error, Result};

// Convert a file name to title case
fn file_auto_title<P: AsRef<Path>>(input: P) -> Option<String> {
    let i = input.as_ref();
    if let Some(nm) = i.file_stem() {
        // If the file is an index file, try to get the name
        // from a parent directory
        if nm == config::INDEX_STEM {
            if let Some(p) = i.parent() {
                return file_auto_title(&p.to_path_buf());
            }
        } else {
            let auto = nm.to_str().unwrap().to_string();
            let capitalized = auto.to_title_case();
            return Some(capitalized);
        }
    }
    None
}

pub fn compute<P: AsRef<Path>>(
    f: P,
    config: &Config,
    opts: &RuntimeOptions,
    frontmatter: bool,
) -> Result<Page> {
    let file = f.as_ref();

    // Start with the global definition
    let mut page = config.page.as_ref().unwrap().clone();

    // Look for file specific data from pages map
    if let Some(ref pages) = config.pages {
        let raw = f.as_ref().to_path_buf();
        if let Ok(rel) = raw.strip_prefix(&opts.source) {
            let file_key = utils::url::to_href_separator(
                rel.to_string_lossy().into_owned(),
            );
            if let Some(file_object) = pages.get(&file_key) {
                let mut copy = file_object.clone();
                page.append(&mut copy);
            }
        } else {
            warn!(
                "Failed to strip prefix for page path {}",
                f.as_ref().display()
            );
        }
    }

    if let None = page.title {
        if let Some(auto) = file_auto_title(&f) {
            page.title = Some(auto);
        }
    }

    if frontmatter {
        let file_type = opts.get_type(f.as_ref());
        let mut conf: frontmatter::Config = Default::default();
        match file_type {
            FileType::Markdown => {
                conf = frontmatter::Config::new_markdown(true)
            }
            FileType::Template => conf = frontmatter::Config::new_html(true),
            _ => {}
        }

        let (_, has_fm, fm) = frontmatter::load(file, conf)?;
        if has_fm {
            parse_into(file, fm, &mut page)?;
        }
    }

    page.compute(config, opts)?;

    Ok(page)
}

fn parse_into<P: AsRef<Path>>(
    file: P,
    source: String,
    data: &mut Page,
) -> Result<()> {
    let mut page: Page = toml::from_str(&source)
        .map_err(|e| Error::FrontMatterParse(file.as_ref().to_path_buf(), e))?;

    data.append(&mut page);
    Ok(())
}

pub fn verify(config: &Config, options: &RuntimeOptions) -> Result<()> {
    if let Some(ref pages) = config.pages {
        for (k, _) in pages {
            let pth = options.source.join(utils::url::to_path_separator(k));
            if !pth.exists() || !pth.is_file() {
                warn!(
                    "Check the [pages.\"{}\"] setting references a file",
                    k.clone()
                );
                warn!(
                    "The file {} has probably been moved or renamed",
                    pth.display()
                );
                return Err(Error::NoPageFile(pth, k.clone()));
            }
        }
    }
    Ok(())
}
