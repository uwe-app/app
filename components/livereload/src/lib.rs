use std::path::PathBuf;
use std::io::Result;

use config::Config;

pub mod messages;

static JS_FILE: &str = "__livereload.js";
static CSS_FILE: &str = "__livereload.css";

static SCRIPT: &str = include_str!("livereload.js");
static CSS: &str = include_str!("livereload.css");

fn get_script(url: &str) -> String {
    // NOTE: we use an IIFE (immediately invoked function expression)
    // NOTE: and the template closes and calls the expression but we
    // NOTE: open it here
    let mut script =
        String::from(format!("(function() {{const socket = new WebSocket('{}');\n", url));
    script.push_str(SCRIPT);
    script
}

fn write_javascript(target: &PathBuf, url: &str) -> Result<()> {
    let mut dest = target.clone();
    dest.push(JS_FILE);
    let script = get_script(url);
    utils::fs::write_string(dest, script)
}

fn write_stylesheet(target: &PathBuf) -> Result<()> {
    let mut dest = target.clone();
    dest.push(CSS_FILE);
    utils::fs::write_string(dest, CSS)
}

/// Get the URL path to the stylesheet.
pub fn stylesheet() -> String {
    format!("/{}", CSS_FILE)
}

/// Get the URL path to the javascript.
pub fn javascript() -> String {
    format!("/{}", JS_FILE)
}

/// Write out the Javascript and CSS files.
pub fn write(
    _config: &Config,
    target: &PathBuf,
    url: &str,
) -> std::io::Result<()> {
    write_javascript(target, url)?;
    write_stylesheet(target)
}
