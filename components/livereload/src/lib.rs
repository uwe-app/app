use std::path::PathBuf;

use config::Config;

pub mod messages;

static SCRIPT: &str = "
socket.onmessage = (event) => {
	const el = document.querySelector('#livereload-notification');
    const e = JSON.parse(event.data);
	if (e.type === 'start') {
		if(el) el.style.display = 'block';
	}else if (e.type === 'reload') {
		socket.close();
		location.reload();
	}
};
window.onbeforeunload = () => socket.close();";

static MARKUP: &str = "
<div id='livereload-notification'
style='
    background: #333;
    color: #cfcfcf;
    position: fixed;
    bottom: 0;
    left: 0;
    font-family: sans-serif;
    font-size: 14px;
    padding: 10px;
    border-top-right-radius: 6px;
    display: none;
'>
<span>Building...</span>
</div>";

fn get_script(url: &str) -> String {
    let mut script = String::from(format!("var socket = new WebSocket('{}')\n", url));
    script.push_str(SCRIPT);
    script
}

pub fn embed(config: &Config) -> String {
    let cfg = config.livereload.as_ref().unwrap();
    let name = cfg.file.as_ref().unwrap().to_string_lossy().into_owned();
    let href = utils::url::to_href_separator(name);
    let notify = cfg.notify.is_some() && cfg.notify.unwrap();

    let mut content = "".to_string();
    if notify {
        content.push_str(MARKUP);
    }
    content.push_str(&format!("<script src=\"/{}\"></script>", href));
    content
}

pub fn write(config: &Config, target: &PathBuf, url: &str) -> std::io::Result<()> {
    let cfg = config.livereload.as_ref().unwrap();
    let mut dest = target.clone();
    dest.push(cfg.file.as_ref().unwrap());
    let script = get_script(url);
    utils::fs::write_string(dest, script)
}

