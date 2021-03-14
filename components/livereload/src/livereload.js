socket.onmessage = (event) => {
    let el = document.querySelector('.livereload-notification');
    if (!el) {
        const inner = document.createElement('span');
        el = document.createElement('div');
        el.appendChild(inner);
        el.classList.add('livereload-notification')
        const body = document.querySelector('body');
        body.appendChild(el);
    }
    const msg = el.querySelector('span');
    const e = JSON.parse(event.data);
	if (e.type === 'start') {
		el.style.display = 'block';
        el.classList.remove('error');
        msg.innerText = 'Building...';
	}else if (e.type === 'reload') {
		socket.close();
    if (e.href) { location.href = e.href; } else { location.reload(); }
	}else if (e.type === 'notify') {
		el.style.display = 'block';
        msg.innerText = e.message;
        if (e.error) {
            console.error(e.message);
            el.classList.add('error');
        }
	}
};

if (window.parent) {
  // Hack so that we can handle history in the editor.
  //
  // It sets the `history` query string parameter to indicate
  // that the navigation is from a history handler (back/forward)
  // so there is no need to notify of the location change.
  const history = new URLSearchParams(
      window.location.search).get('history');
  if (!history) {
    window.parent.postMessage(document.location.href, "*");
  }
}

window.onbeforeunload = () => socket.close()})();
