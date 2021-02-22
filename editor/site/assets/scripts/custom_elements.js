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

