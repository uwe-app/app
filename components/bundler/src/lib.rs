#[macro_use]
extern crate log;

mod bundler;
mod command;

pub use command::BundleOptions;
pub use command::bundle;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BundleError {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Git(#[from] git::error::GitError),

    #[error(transparent)]
    Cache(#[from] cache::CacheError),

    #[error(transparent)]
    Preference(#[from] preference::PreferenceError),
}

impl BundleError {
    pub fn new(s: String) -> Self {
        BundleError::Message(s) 
    }
}


//#[cfg(test)]
//mod tests {
    //#[test]
    //fn it_works() {
        //assert_eq!(2 + 2, 4);
    //}
//}
