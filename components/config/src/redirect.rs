use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::Path;

use log::info;

use http::Uri;

use crate::{Error, Result, RuntimeOptions};

static MAX_REDIRECTS: usize = 4;
pub static REDIRECTS_FILE: &str = "redirects.json";

pub type Redirects = HashMap<String, Uri>;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RedirectConfig {
    #[serde(flatten)]
    manifest: RedirectManifest,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RedirectManifest {
    #[serde(flatten)]
    map: HashMap<String, String>,
}

impl RedirectManifest {
    pub fn map(&self) -> &HashMap<String, String> {
        &self.map 
    }

    pub fn map_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.map 
    }
}

impl TryInto<Redirects> for RedirectConfig {
    type Error = crate::Error;

    fn try_into(self) -> std::result::Result<Redirects, Self::Error> {
        let mut map: HashMap<String, Uri> = HashMap::new();
        for (k, v) in self.manifest.map {
            map.insert(k, v.as_str().parse::<Uri>()?);
        }
        Ok(map)
    }
}

impl RedirectConfig {
    pub fn map(&self) -> &HashMap<String, String> {
        self.manifest.map()
    }

    pub fn map_mut(&mut self) -> &mut HashMap<String, String> {
        self.manifest.map_mut()
    }

    pub fn validate(&self) -> Result<()> {
        for (k, v) in self.map() {
            let mut stack: Vec<String> = Vec::new();
            self.validate_redirect(k, v, &mut stack)?;
        }
        Ok(())
    }

    // FIXME: improve this redirect validation logic to handle
    // FIXME: trailing slashes on sources and targets better
    fn validate_redirect<S: AsRef<str>, T: AsRef<str>>(
        &self,
        k: S,
        v: T,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        if stack.len() >= MAX_REDIRECTS {
            return Err(Error::TooManyRedirects(MAX_REDIRECTS));
        }

        let mut key = k.as_ref().to_string().clone();
        key = key.trim_end_matches("/").to_string();

        if stack.contains(&key) {
            return Err(Error::CyclicRedirect {
                stack: stack.join(" <-> "),
                key: key.clone(),
            });
        }

        stack.push(key);

        // Check raw value first
        if let Some(value) = self.manifest.map.get(v.as_ref()) {
            return self.validate_redirect(v.as_ref(), value, stack);
        }

        // Try with a trailing slash removed
        let mut val_key = v.as_ref().to_string();
        val_key = val_key.trim_end_matches("/").to_string();
        if let Some(value) = self.manifest.map.get(&val_key) {
            return self.validate_redirect(&val_key, value, stack);
        }

        Ok(())
    }

    pub fn write(&self, options: &RuntimeOptions) -> Result<()> {
        let build_target = options.build_target();
        if options.settings.write_redirect_files() {
            self.create_files(build_target)?;
        }
        self.create_json(build_target)?;
        Ok(())
    }

    fn create_json<P: AsRef<Path>>(&self, target: P) -> Result<()> {
        info!(
            "Write {} redirect(s) to {}",
            self.map().len(),
            target.as_ref().display()
        );
        let target = target.as_ref().join(REDIRECTS_FILE);
        fs::write(target, serde_json::to_vec(&self.manifest)?)?;
        Ok(())
    }

    fn create_files<P: AsRef<Path>>(&self, target: P) -> Result<()> {
        let target = target.as_ref();
        for (k, v) in self.map() {
            // Strip the trailing slash so it is not treated
            // as an absolute path on UNIX
            let key = k.trim_start_matches("/");
            let mut buf = target.join(utils::url::to_path_separator(key));
            if k.ends_with("/") {
                buf.push(crate::INDEX_HTML);
            }

            if buf.exists() {
                return Err(Error::RedirectFileExists(buf));
            }

            let short = buf.strip_prefix(target)?;
            info!("{} -> {} as {}", &k, &v, short.display());
            if let Some(ref parent) = buf.parent() {
                fs::create_dir_all(parent)?;
            }
            self.write_file(&v, &buf)?;
        }
        Ok(())
    }

    fn write_file<P: AsRef<Path>>(
        &self,
        location: &str,
        target: P,
    ) -> std::io::Result<()> {
        let mut content = String::from("<!doctype html>");
        let body = format!(
            "<body onload=\"document.location.replace('{}');\"></body>",
            location
        );
        let meta = format!(
            "<noscript><meta http-equiv=\"refresh\" content=\"0; {}\"></noscript>",
            location
        );
        content.push_str("<html>");
        content.push_str("<head>");
        content.push_str(&format!(
            "<link rel=\"canonical\"  href=\"{}\">",
            location
        ));
        content.push_str(&meta);
        content.push_str("</head>");
        content.push_str(&body);
        content.push_str("</html>");
        utils::fs::write_string(target, content)
    }
}
