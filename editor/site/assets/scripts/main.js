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
