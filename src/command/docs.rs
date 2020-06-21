use crate::preference;
use crate::cache::{self, CacheComponent};
use crate::Error;

#[derive(Debug)]
pub struct DocsOptions {
    pub host: String,
    pub port: u16,
}

pub fn docs(options: DocsOptions) -> Result<(), Error> {
    println!("Cache docs repository");
    println!("Serve docs directory");
    Ok(())
}
