import {h, createRef} from 'preact';
import {useEffect} from 'preact/hooks';
import {schema, defaultMarkdownParser,
        defaultMarkdownSerializer} from "prosemirror-markdown"
import {EditorState} from "prosemirror-state";
import {EditorView} from "prosemirror-view";
import {exampleSetup} from "prosemirror-example-setup";

const Header = (props) => {
  return <header>File editor actions</header>;
}

const Content = (props) => {
  const view = createRef();

  useEffect(() => {
    if (view.current) {
      let content = "## Some title";

      let state = EditorState.create({
        doc: defaultMarkdownParser.parse(content),
        plugins: exampleSetup({schema})
      });

      let editor = new EditorView(view.current, {state});
      editor.focus();
    }
  }, []);

  return <div class="content">
    <div ref={view}></div>
  </div>;
}

const Footer = (props) => {
  return <footer>...</footer>;
}

export default function FileEditor(props) {
  return <div class="file-editor">
    <Header />
    <Content />
    <Footer />
  </div>;
}
