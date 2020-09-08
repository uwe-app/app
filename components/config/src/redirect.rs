use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::info;

use http::Uri;

use crate::{Error, Result, RuntimeOptions};

static MAX_REDIRECTS: usize = 4;

pub type Redirects = HashMap<String, Uri>;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RedirectConfig {
    #[serde(flatten)]
    pub map: HashMap<String, String>,
}

impl RedirectConfig {
    pub fn collect(&self) -> crate::Result<Redirects> {
        let mut map: HashMap<String, Uri> = HashMap::new();
        for (k, v) in self.map.iter() {
            map.insert(k.clone(), v.as_str().parse::<Uri>()?);
        }
        Ok(map)
    }

    pub fn validate(&self) -> Result<()> {
        for (k, v) in self.map.iter() {
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
        if let Some(value) = self.map.get(v.as_ref()) {
            return self.validate_redirect(v.as_ref(), value, stack);
        }

        // Try with a trailing slash removed
        let mut val_key = v.as_ref().to_string();
        val_key = val_key.trim_end_matches("/").to_string();
        if let Some(value) = self.map.get(&val_key) {
            return self.validate_redirect(&val_key, value, stack);
        }

        Ok(())
    }

    pub fn write(&self, options: &RuntimeOptions) -> Result<()> {
        let write_redirects = options.settings.write_redirects.is_some()
            && options.settings.write_redirects.unwrap();
        if write_redirects {
            self.write_all(&options.base)?;
        }
        Ok(())
    }

    fn write_all<P: AsRef<Path>>(&self, target: P) -> Result<()> {
        for (k, v) in self.map.iter() {
            // Strip the trailing slash so it is not treated
            // as an absolute path on UNIX
            let key = k.trim_start_matches("/");
            let mut buf = target.as_ref().to_path_buf();
            buf.push(utils::url::to_path_separator(key));
            if k.ends_with("/") {
                buf.push(crate::INDEX_HTML);
            }
            if buf.exists() {
                return Err(Error::RedirectFileExists(buf));
            }

            let short = buf.strip_prefix(target.as_ref())?;
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
        content.push_str(&meta);
        content.push_str("</head>");
        content.push_str(&body);
        content.push_str("</html>");
        utils::fs::write_string(target, content)
    }
}
