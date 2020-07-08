use std::path::PathBuf;

use config::Config;

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

static STYLE: &str = "
<style>
#livereload-notification {
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
}
</style>";

static MARKUP: &str = "
    <div id='livereload-notification'><span>Building...</span></div>
";

fn get_script(url: &str) -> String {
    let mut script = String::from(format!("var socket = new WebSocket('{}')\n", url));
    script.push_str(SCRIPT);
    script
}

pub fn embed(config: &Config) -> String {
    let cfg = config.livereload.as_ref().unwrap();
    let name = cfg.file.as_ref().unwrap().to_string_lossy().into_owned();
    let href = utils::url::to_href_separator(name);

    let mut content = "".to_string();
    content.push_str(STYLE);
    content.push_str(MARKUP);
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

