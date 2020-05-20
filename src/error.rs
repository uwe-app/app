use std::{fmt,io,error};

use handlebars;
use ignore;
use mdbook;

#[derive(Debug)]
pub enum Error {
    Message(String),
    IoError(io::Error),
    TemplateFileError(handlebars::TemplateFileError),
    IgnoreError(ignore::Error),
    BookError(mdbook::errors::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Message(ref s) =>  write!(f,"{}", s),
            Error::IoError(ref e) => e.fmt(f),
            Error::TemplateFileError(ref e) => e.fmt(f),
            Error::IgnoreError(ref e) => e.fmt(f),
            Error::BookError(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::IoError(ref e) => Some(e),
            Error::TemplateFileError(ref e) => Some(e),
            Error::IgnoreError(ref e) => Some(e),
            Error::BookError(ref e) => Some(e),
            _ => None
        }
    }
}

