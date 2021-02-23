const isEmbedded = typeof external_handler === 'function';
if (isEmbedded) {
  console.info = function() {
    log_info.apply(null, Array.prototype.slice.call(arguments));
  }
  console.log = function() {
    log_info.apply(null, Array.prototype.slice.call(arguments));
  }
  console.warn = function() {
    log_warn.apply(null, Array.prototype.slice.call(arguments));
  }
  console.error = function() {
    log_error.apply(null, Array.prototype.slice.call(arguments));
  }
}

window.onerror = function(message, filename, lineno, colno, error) {
    if (error != null) {
      alert(error.stack);
    } else {
      alert(`${message} ${filename} ${lineno}`);
    }
};

class JsonRpc {
  request(method, params) {
    const req = {
      jsonrpc: "2.0",
      id: Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
      method,
      params
    };
    return req;
  }
}

class WebViewIpc {
  constructor(call, rpc) {
    this.external = {call};
    this.rpc = rpc;
    this.fullscreen = false;
    this.responses = {};
  }

  toggleFullScreen() {
    const res = this.send('window.set_fullscreen', !this.fullscreen);
    this.fullscreen = !this.fullscreen;
    return res;
  }

  openFolder(title) {
    return this.send('folder.open', title);
  }

  openProject(path) {
    return this.send('project.open', [path]);
  }

  send(method, params) {
    const request = this.rpc.request(method, params);
    const req = JSON.stringify(request);
    const id = request.id;
    const p = new Promise((resolve, reject) => {
      const poll = setInterval(() => {
        const message = this.responses[id];
        if (message) {
          delete this.responses[id];
          clearInterval(poll);
          if (!message.error) {
            resolve(message.result);
          } else {
            reject(message.error);
          }
        }
      }, 5);
    });
    this.external.call(req);
    return p;
  }
}

if (typeof external_handler === 'function') {
  window.ipc = new WebViewIpc(external_handler, new JsonRpc());
}

function onIpcMessage(message) {
  document.getElementById("ipc-result").innerHTML =
    JSON.stringify(message, undefined, 2);
}

async function chooseProject() {
  const path = await ipc.openFolder('Choose a project');
  alert('Folder path ' + path);
}

//console.info('App started...');
//console.log('App started...');
//console.warn('App started...');
//console.error('App started...');
