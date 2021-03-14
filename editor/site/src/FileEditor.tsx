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

const Content = ({ webdav, file }) => {
  const view = createRef();

  useEffect(async () => {
    if (view.current) {
      let content = "";
      if (file) {
        const content = await webdav.getFileContents(
            file.filename, {format: 'text'});

        while (view.current.firstChild !== null) {
          view.current.removeChild(view.current.firstChild);
        }

        let state = EditorState.create({
          doc: defaultMarkdownParser.parse(content),
          plugins: exampleSetup({schema})
        });

        let editor = new EditorView(view.current, {state});
        editor.focus();

      }
    }
  });

  return <div class="content">
    <div ref={view}></div>
  </div>;
}

const Footer = ({ file }) => {
  if (file) {
    return <footer class="no-select">
      <small>{file.filename} ({file.size})</small>
    </footer>;
  }
  return null;
}

export default function FileEditor({ webdav, file }) {
  return <div class="file-editor">
    <Header />
    <Content webdav={webdav} file={file} />
    <Footer file={file} />
  </div>;
}
