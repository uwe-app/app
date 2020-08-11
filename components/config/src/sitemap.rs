use std::io;
use serde::{Deserialize, Serialize};

use url::Url;

pub static FILE: &str = "index.xml";
pub static NAME: &str = "sitemap";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct SiteMapConfig {
    // Number of entries per sitemap file
    pub entries: Option<usize>,
    // The folder name used to put the files
    pub name: Option<String>,
}

impl Default for SiteMapConfig {
    fn default() -> Self {
        Self {
            entries: Some(25000),
            name: Some(NAME.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct SiteMapIndex {
    pub base: Url,
    pub maps: Vec<SiteMapFile>,
    pub folder: String,
}

impl SiteMapIndex {

    pub fn new(base: Url, folder: String) -> Self {
        Self {base, folder, maps: vec![]}
    }

    pub fn to_location(&self) -> Url {
        let path = self.base.path();
        // This handles multi-lingual base paths correctly
        let dest = if path == "/" {
            format!("{}/{}", &self.folder, FILE)
        } else {
            format!("{}/{}/{}", path, &self.folder, FILE)
        };
        self.base.join(&dest).unwrap()
    }

    pub fn to_writer<W>(&self, mut w: W) -> io::Result<()> where W: io::Write {
        write!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        write!(w, "<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n")?;
        for map in self.maps.iter() {
            write!(w, "\t<sitemap>\n")?;
            let loc = utils::entity::escape(&map.to_location(&self.to_location()).to_string());
            write!(w, "\t\t<loc>{}</loc>\n", loc)?;
            write!(w, "\t</sitemap>\n")?;
        }
        write!(w, "</sitemapindex>")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SiteMapFile {
    pub href: String,
    pub entries: Vec<SiteMapEntry>,
}

impl SiteMapFile {
    pub fn to_writer<W>(&self, mut w: W) -> io::Result<()> where W: io::Write {
        write!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
        write!(w, "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n")?;
        for entry in self.entries.iter() {
            write!(w, "\t<url>\n")?;
            let loc = utils::entity::escape(&entry.location.to_string());
            let lastmod = utils::entity::escape(&entry.lastmod);
            write!(w, "\t\t<loc>{}</loc>\n", loc)?;
            write!(w, "\t\t<lastmod>{}</lastmod>\n", lastmod)?;
            write!(w, "\t</url>\n")?;
        }
        write!(w, "</urlset>")?;
        Ok(())
    }

    pub fn to_location(&self, base: &Url) -> Url {
        base.join(&self.href).unwrap()
    }
}

#[derive(Debug)]
pub struct SiteMapEntry {
    pub location: Url,
    pub lastmod: String,
}
