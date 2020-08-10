use std::io;
use serde::{Deserialize, Serialize};

use url::Url;

pub static FILE: &str = "sitemap.xml";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SiteMapConfig {
    pub entries: Option<usize>,
}

impl Default for SiteMapConfig {
    fn default() -> Self {
        Self {
            entries: Some(10000),
        }
    }
}

#[derive(Debug)]
pub struct SiteMapIndex {
    pub base: Url,
    pub maps: Vec<SiteMapFile>,
}

impl SiteMapIndex {

    pub fn new(base: Url) -> Self {
        Self {base, maps: vec![]}
    }

    pub fn create(&self, count: usize) -> SiteMapFile {
        let name: String = format!("sitemap-{}.xml", count + 1); 
        SiteMapFile {name, entries: vec![]}
    }

    pub fn to_writer<W>(&self, mut w: W) -> io::Result<()> where W: io::Write {
        write!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        write!(w, "<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n")?;
        for map in self.maps.iter() {
            write!(w, "<sitemap>\n")?;
            let loc = utils::entity::escape(&map.to_location(&self.base).to_string());
            write!(w, "<loc>{}</loc>", loc)?;
            write!(w, "</sitemap>\n")?;
        }
        write!(w, "</sitemapindex>")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SiteMapFile {
    pub name: String,
    pub entries: Vec<SiteMapEntry>,
}

impl SiteMapFile {
    pub fn to_writer<W>(&self, mut w: W) -> io::Result<()> where W: io::Write {
        write!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        write!(w, "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n")?;
        for entry in self.entries.iter() {
            write!(w, "<url>\n")?;
            let loc = utils::entity::escape(&entry.location.to_string());
            write!(w, "<loc>{}</loc>", loc)?;
            write!(w, "</url>\n")?;
        }
        write!(w, "</urlset>")?;
        Ok(())
    }

    pub fn to_location(&self, base: &Url) -> Url {
        base.join(&self.name).unwrap()
    }
}

#[derive(Debug)]
pub struct SiteMapEntry {
    pub location: Url,
    pub lastmod: String,
}
