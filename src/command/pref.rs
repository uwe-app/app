use crate::preference::{self, Preferences};
use crate::{utils, Error};

use log::warn;

#[derive(Debug)]
pub struct PrefOptions {
    pub edit: bool,
}

pub fn edit(content: Option<String>) -> Result<(), Error> {
    preference::init_if_none()?;

    let prefs = if let Some(file_content) = content {
        file_content
    } else {
        preference::load_file()?
    };

    let result = edit::edit(prefs)?;
    let valid = toml::from_str::<Preferences>(&result);
    match valid {
        Ok(_new_prefs) => {
            utils::fs::write_string(preference::get_prefs_file()?, result)?;
            return Ok(());
        }
        Err(e) => {
            warn!("{}", e);
            return edit(Some(result));
        }
    }
}

pub fn pref(options: PrefOptions) -> Result<(), Error> {
    if options.edit {
        return edit(None);
    }
    Ok(())
}
