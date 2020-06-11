use std::path::Path;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

use serde_json::{Map, Value};

use crate::Error;
use super::loader;

type ContentResult = (String, bool, String);

pub struct Config {
    pub start: String,
    pub end: String,
    // Use bail when you only want the front matter
    pub bail: bool,
}

impl Config {
    pub fn new_markdown(bail: bool) -> Self {
        Self {
            start: String::from("+++"),
            end: String::from("+++"),
            bail,
        }
    }

    pub fn new_html(bail: bool) -> Self {
        Self {
            start: String::from("<!--"),
            end: String::from("-->"),
            bail,
        }
    }
}

pub fn split<P: AsRef<Path>>(p: P, conf: Config) -> Result<ContentResult, Error> {

    let mut fm = String::new();
    let mut content = String::new();
    let mut in_front_matter = false;
    let mut has_front_matter = false;

    let f = File::open(p.as_ref())?;
    let reader = BufReader::new(f);

    let newline = if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    };

    for line in reader.lines() {
        match line {
            Ok(line) => {

                if in_front_matter && line.trim() == conf.end {
                    in_front_matter = false;
                    if conf.bail {
                        return Ok((content, has_front_matter, fm))
                    }
                    continue;
                }

                if in_front_matter {
                    fm.push_str(&line); 
                    fm.push_str(newline);
                    continue;
                }

                if !has_front_matter && line.trim() == conf.start {
                    in_front_matter = true;
                    has_front_matter = true;
                    continue;
                }

                // Always respect bail, it tells us to never read the
                // actual file content as we only want to extract the 
                // front matter data
                if conf.bail {
                    return Ok((content, has_front_matter, fm))
                }

                content.push_str(&line);
                content.push_str(newline);

            },
            Err(e) => return Err(Error::from(e)),
        }
    }

    if in_front_matter {
        return Err(
            Error::new(
                format!("Front matter was not terminated")));
    }

    return Ok((content, has_front_matter, fm))
}

