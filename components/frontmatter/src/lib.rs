use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

const HTML: &str = "html";

/// Get aconfiguration depending upon a path file extension.
pub fn get_config(file: &PathBuf) -> Config {
    if let Some(ext) = file.extension() {
        if ext == HTML {
            return Config::new_html(false);
        }
    }
    Config::new_markdown(false)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Front matter was not terminated in {0}")]
    NotTerminated(PathBuf),
}

type ContentResult = (String, bool, String);

#[derive(Debug, Default)]
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

// Pads the content with lines so that template
// error messages with line numbers are correct.
pub fn load<P: AsRef<Path>>(
    p: P,
    conf: Config,
) -> Result<ContentResult, Error> {
    let mut fm = String::new();
    let mut content = String::new();
    let mut in_front_matter = false;
    let mut has_front_matter = false;

    let f = File::open(p.as_ref())?;
    let reader = BufReader::new(f);

    let newline = if cfg!(windows) { "\r\n" } else { "\n" };

    for line in reader.lines() {
        match line {
            Ok(line) => {
                if in_front_matter && line.trim() == conf.end {
                    content.push_str(newline);
                    in_front_matter = false;
                    if conf.bail {
                        return Ok((content, has_front_matter, fm));
                    }
                    continue;
                }

                if in_front_matter {
                    content.push_str(newline);
                    fm.push_str(&line);
                    fm.push_str(newline);
                    continue;
                }

                if !has_front_matter
                    && line.trim() == conf.start
                    && content.is_empty()
                {
                    content.push_str(newline);
                    in_front_matter = true;
                    has_front_matter = true;
                    continue;
                }

                // Always respect bail, it tells us to never read the
                // actual file content as we only want to extract the
                // front matter data
                if conf.bail {
                    return Ok((content, has_front_matter, fm));
                }

                content.push_str(&line);
                content.push_str(newline);
            }
            Err(e) => return Err(Error::from(e)),
        }
    }

    if in_front_matter {
        return Err(Error::NotTerminated(p.as_ref().to_path_buf()));
    }

    return Ok((content, has_front_matter, fm));
}
