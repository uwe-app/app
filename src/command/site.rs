use std::path::PathBuf;

use crate::Result;

#[derive(Debug)]
pub struct AddOptions {
    pub name: String,
    pub project: PathBuf,
}

#[derive(Debug)]
pub struct RemoveOptions {
    pub name: String,
}

#[derive(Debug)]
pub struct ListOptions {}

pub fn add(options: AddOptions) -> Result<()> {
    println!("Add site: {:?}", options);
    Ok(())
}

pub fn remove(options: RemoveOptions) -> Result<()> {
    println!("Remove site: {:?}", options);
    Ok(())
}

pub fn list(_options: ListOptions) -> Result<()> {
    println!("List sites");
    Ok(())
}
