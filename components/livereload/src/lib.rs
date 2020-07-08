use std::path::PathBuf;

use config::Config;

static LIVERELOAD_NAME: &str = "__livereload.js";

static SCRIPT: &str = "
socket.onmessage = (event) => {
	const el = document.querySelector('#livereload-notification');
	if (event.data === 'start') {
		if(el) el.style.display = 'block';
	}else if (event.data === 'reload') {
		socket.close();
		location.reload();
	}
};
window.onbeforeunload = () => socket.close();";

static STYLE: &str = "#livereload-notification {
    background: black;
    color: white;
    z-index: 999991;
    position: fixed;
    bottom: 0;
    left: 0;
    font-family: sans-serif;
    font-size: 14px;
    padding: 10px;
    border-top-right-radius: 6px;
    display: none;
}";

fn get_script(url: &str) -> String {
    let mut script = String::from(format!("var socket = new WebSocket('{}')\n", url));
    script.push_str(SCRIPT);
    script
}

pub fn embed(_config: &Config) -> String {
    let mut content = "".to_string();
    content.push_str(&format!("<style>{}</style>", STYLE));
    content.push_str("<div id='livereload-notification'><span>Building...</span></div>");
    content.push_str("<script src=\"/__livereload.js\"></script>");
    content
}

pub fn write(_config: &Config, target: &PathBuf, url: &str) -> std::io::Result<()> {
    let mut dest = target.clone();
    dest.push(LIVERELOAD_NAME);
    let script = get_script(url);
    utils::fs::write_string(dest, script)
}

