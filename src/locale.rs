use std::path::Path;
use std::fmt;

use fluent_templates::ArcLoader;

use crate::Config;
use crate::config::FluentConfig;
use crate::Error;

pub struct Locales {
    pub loader: Option<ArcLoader>,
}

impl Default for Locales {
    fn default() -> Self {
        Self {loader: None}
    }
}

impl fmt::Debug for Locales {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").finish()
    }
}

impl Locales {
    pub fn load<P: AsRef<Path>>(&mut self, config: &Config, source: P) -> Result<(), Error> {

        if let Some(locales_dir) = config.get_locales(source) {
            if locales_dir.exists() && locales_dir.is_dir() {
                let fluent = config.fluent.as_ref().unwrap();
                // FIXME: catch and return this error
                self.loader = Some(self.arc(locales_dir, fluent).unwrap());
            }
        }
        Ok(())
    }

    fn arc<'a, P: AsRef<Path>>(&mut self, dir: P, fluent: &FluentConfig)
        -> Result<ArcLoader, Box<dyn std::error::Error>> {

        let file = dir.as_ref();
        if let Some(core_file) = &fluent.shared {
            let mut core = file.to_path_buf();
            core.push(core_file);
            return ArcLoader::builder(dir.as_ref(), fluent.fallback_id.clone())
                .shared_resources(Some(&[core])).build();
        }

        ArcLoader::builder(dir.as_ref(), fluent.fallback_id.clone()).build()
            //.customize(|bundle| bundle.set_use_isolating(false));
    }
}

