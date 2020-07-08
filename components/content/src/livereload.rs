use std::path::PathBuf;

use utils;

static LIVERELOAD_NAME: &str = "__livereload.js";

static SCRIPT: &str = "
socket.onmessage = (event) => {
	const el = document.querySelector('#livereload-notification');
	if (event.data === 'start') {
		if(el) el.style.display = 'block';
	}else if (event.data === 'reload') {
		console.log('got reload notification');socket.close();
		location.reload();
	}
};
window.onbeforeunload = () => socket.close();";

fn get_script(url: &str) -> String {
    let mut script = String::from(format!("var socket = new WebSocket('{}')\n", url));
    script.push_str(SCRIPT);
    script
}

pub fn write(target: &PathBuf, url: &str) -> std::io::Result<()> {
    let mut dest = target.clone();
    dest.push(LIVERELOAD_NAME);
    let script = get_script(url);
    utils::fs::write_string(dest, script)
}
