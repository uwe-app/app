import {h, createRef} from 'preact';
import {useEffect, useState} from 'preact/hooks';
import {schema, defaultMarkdownParser} from "prosemirror-markdown"
import {EditorState} from "prosemirror-state";
import {EditorView} from "prosemirror-view";

import {setup} from "./prosemirror/index";
import {defaultMarkdownSerializer} from './prosemirror/markdown-serializer';
import humanFileSize from './utils/file-size';

const Header = ({ file, onSave }) => {

  if (file) {
    return <header>
      <span>{file.basename}</span>
      <button onclick={(e) => onSave(e, file)}>Save</button>
    </header>;
  }
  return null;
}

const Content = ({ webdav, file, setEditor }) => {

  //console.log('Content rendering ', file);

  const view = createRef();

  useEffect(async () => {
    if (view.current && file) {
      const content = await webdav.getFileContents(
          file.filename, {format: 'text'});
      while (view.current.firstChild !== null) {
        view.current.removeChild(view.current.firstChild);
      }

      let state = EditorState.create({
        doc: defaultMarkdownParser.parse(content),
        plugins: setup({schema})
      });

      let editor = new EditorView(view.current, {state});
      editor.focus();

      setEditor({state, view: editor})
    }
  }, [file]);

  return <div class="content">
    <div ref={view}></div>
  </div>;
}

const Footer = ({ file }) => {
  if (file) {
    return <footer class="no-select">
      <small>{file.filename} ({humanFileSize(file.size)})</small>
    </footer>;
  }
  return null;
}

export default function FileEditor({ webdav, file }) {
  const [editor, setEditor] = useState(null);

  const onSave = async (e, file) => {
    e.preventDefault();
    if (editor) {
      const content = defaultMarkdownSerializer.serialize(editor.view.state.doc);
      //console.log(content);
      //console.log('Save file', file.filename);
      const result = await webdav.putFileContents(file.filename, content);
    }
  }

  if (file) {
    return <div class="file-editor">
      <Header file={file} onSave={onSave} />
      <Content webdav={webdav} file={file} setEditor={setEditor} />
      <Footer file={file} />
    </div>;
  }
}
