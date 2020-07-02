use thiserror::Error;

use config::Config;

use log::info;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Book(#[from] mdbook::errors::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

type Result<T> = std::result::Result<T, Error>;

// List books in the project
pub fn list(config: &Config) -> Result<()> {
    info!("List books in {}", config.get_project().display());
    if let Some(ref book) = config.book {
        if !book.members.is_empty() {
            for (group, members) in &book.members {
                for (name, cfg) in members {
                    info!("{}.{} -> {}", group, name, cfg.path.display());
                }
            }
            return Ok(())
        }
    }
    info!("No books yet");
    Ok(())
}

// Create a new book
pub fn add(config: &Config) -> Result<()> {
    Ok(())
}

//#[cfg(test)]
//mod tests {
    //#[test]
    //fn it_works() {
        //assert_eq!(2 + 2, 4);
    //}
//}
