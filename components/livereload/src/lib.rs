use std::path::PathBuf;

use config::Config;

pub mod messages;

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

static MARKUP: &str = "
<style>
.livereload-notification {
    max-width: 640px;
    background: #333;
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

