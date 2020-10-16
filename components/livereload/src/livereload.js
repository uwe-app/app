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
window.onbeforeunload = () => socket.close()})();
