class JsonRpc {
  constructor() {
    this.id = 0;
  }

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
  }

  openFolder() {
    this.send('folder.open')
  }

  openProject(path) {
    this.send('project.open', [path])
  }

  send(method, params) {
    const req = JSON.stringify(this.rpc.request(method, params));
    this.external.call(req);
  }
}

if (typeof onIpcRequest === 'function') {
  window.ipc = new WebViewIpc(onIpcRequest, new JsonRpc());
}

class EditorView extends HTMLElement {
  constructor() {
    super();

    if (this.shadowRoot) {
      // A Declarative Shadow Root exists!
      // wire up event listeners, references, etc.:
      const button = this.shadowRoot.firstElementChild;
      button.addEventListener('click', () => {
        console.log('Toggle was called...');
      });
    } else {
      console.error("Component does not have a shadow root");
    }
  }
}

// Polyfill for declarative shadow dom
document.querySelectorAll('template[shadowroot]').forEach(template => {
  const mode = template.getAttribute('shadowroot');
  const shadowRoot = template.parentNode.attachShadow({ mode });
  shadowRoot.appendChild(template.content);
  template.remove();
});

// Register custom elements
customElements.define('editor-view', EditorView);

let fullscreen = false;

function toggleFullScreen() {
    if (fullscreen) {
        onIpcRequest(JSON.stringify({event: 'exit-fullscreen'}));
    } else {
        onIpcRequest(JSON.stringify({event: 'enter-fullscreen'}));
    }
    fullscreen = !fullscreen;
}

function onIpcMessage(message) {
    document.getElementById("ipc-result").innerHTML = JSON.stringify(message, undefined, 2);
}
