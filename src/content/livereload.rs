use crate::utils;
use crate::Error;
use std::path::PathBuf;

use log::debug;

static LIVERELOAD_NAME: &str = "__livereload.js";

fn get_script(url: &str) -> String {
    let mut script = String::from(format!("var socket = new WebSocket('{}')\n", url));
    script.push_str("socket.onmessage = (event) => {\n");
    script.push_str("\tif (event.data === 'reload') {\n");
    script.push_str("\t\tsocket.close();\n");
    script.push_str("\t\tlocation.reload();\n");
    script.push_str("\t}\n");
    script.push_str("};\n");
    script.push_str("window.onbeforeunload = () => socket.close();");
    script
}

pub fn write(target: &PathBuf, url: &str) -> Result<(), Error> {
    let mut dest = target.clone();
    dest.push(LIVERELOAD_NAME);
    let script = get_script(url);
    debug!("{} {}", dest.display(), url);
    debug!("{}", script);
    utils::write_string(dest, script).map_err(Error::from)
}
