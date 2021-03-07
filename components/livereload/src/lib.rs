use std::io::Result;
use std::path::PathBuf;

use config::Config;

pub mod messages;

const JS_FILE: &str = "__livereload.js";
const CSS_FILE: &str = "__livereload.css";

const SCRIPT: &str = include_str!("livereload.js");
const CSS: &str = include_str!("livereload.css");

fn get_script(endpoint: &str) -> String {
    // NOTE: we use an IIFE (immediately invoked function expression)
    // NOTE: and the template closes and calls the expression but we
    // NOTE: open it here
    let mut script = String::from(format!(
        "(function() {{
            const protocol = document.location.protocol === 'https:' ? 'wss' : 'ws';
            const url = `${{protocol}}://${{document.location.host}}/{}`;
            const socket = new WebSocket(url);\n",
        endpoint
    ));
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
    endpoint: &str,
) -> std::io::Result<()> {
    write_javascript(target, endpoint)?;
    write_stylesheet(target)
}
