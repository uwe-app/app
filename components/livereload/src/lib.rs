use std::path::PathBuf;

use config::Config;

pub mod messages;

static JS_FILE: &str = "__livereload.js";
static CSS_FILE: &str = "__livereload.css";

static SCRIPT: &str = "
socket.onmessage = (event) => {
	const el = document.querySelector('.livereload-notification');
    const msg = el.querySelector('span');
    const e = JSON.parse(event.data);
	if (e.type === 'start') {
		el.style.display = 'block';
        el.classList.remove('error');
        msg.innerText = 'Building...';
	}else if (e.type === 'reload') {
		socket.close();
		location.reload();
	}else if (e.type === 'notify') {
		el.style.display = 'block';
        msg.innerText = e.message;
        if (e.error) {
            console.error(e.message);
            el.classList.add('error');
        }
	}
};
window.onbeforeunload = () => socket.close();";

static CSS: &str = "
.livereload-notification {
    max-width: 640px;
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
}
.livereload-notification.error {
    background: #933;
}";

static MARKUP: &str = "
<style>
.livereload-notification {
    max-width: 640px;
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
}
.livereload-notification.error {
    background: #933;
}
</style>
<div class='livereload-notification'
style='
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
<span></span>
</div>";

fn get_script(url: &str) -> String {
    let mut script =
        String::from(format!("var socket = new WebSocket('{}')\n", url));
    script.push_str(SCRIPT);
    script
}

pub fn embed(config: &Config) -> String {
    let cfg = config.livereload.as_ref().unwrap();
    let name = JS_FILE;
    let notify = cfg.notify.is_some() && cfg.notify.unwrap();

    let mut content = "".to_string();
    if notify {
        content.push_str(MARKUP);
    }
    content.push_str(&format!("<script src=\"/{}\"></script>", name));
    content
}

fn write_javascript(
    config: &Config,
    target: &PathBuf,
    url: &str,
) -> std::io::Result<()> {
    let mut dest = target.clone();
    dest.push(JS_FILE);
    let script = get_script(url);
    utils::fs::write_string(dest, script)
}

fn write_stylesheet(
    config: &Config,
    target: &PathBuf
) -> std::io::Result<()> {
    let mut dest = target.clone();
    dest.push(CSS_FILE);
    utils::fs::write_string(dest, CSS)
}

pub fn write(
    config: &Config,
    target: &PathBuf,
    url: &str,
) -> std::io::Result<()> {
    write_javascript(config, target, url)?;
    write_stylesheet(config, target)
}
