use crate::Error;
use crate::preference;

#[derive(Debug)]
pub struct PrefOptions {
    pub init: bool,
}

pub fn pref(options: PrefOptions) -> Result<(), Error> {
    if options.init {
        return preference::init()
    }
    Ok(())
}
