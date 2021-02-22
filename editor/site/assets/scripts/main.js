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
    this.external.call(req);
  }
}

if (typeof onIpcRequest === 'function') {
  window.ipc = new WebViewIpc(onIpcRequest, new JsonRpc());
}

function onIpcMessage(message) {
    document.getElementById("ipc-result").innerHTML = JSON.stringify(message, undefined, 2);
}
