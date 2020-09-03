use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use http::Uri;

use crate::{Error, Result};

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
    fn validate_redirect<S: AsRef<str>>(
        &self,
        k: S,
        v: S,
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
}

