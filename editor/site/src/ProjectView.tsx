import {h} from 'preact';
import {useEffect, useContext, useState} from 'preact/hooks';
import {useRoute, useLocation} from 'wouter';
import {Link} from 'wouter';
import {createClient} from 'webdav/web';

import {State} from './State'
import FileManager from './FileManager';
import FileEditor from './FileEditor';
import WebsitePreview from './WebsitePreview';

export default function ProjectView() {
  const state = useContext(State);
  const [location, setLocation] = useLocation();

  const [match, params] = useRoute("/project/:id");
  const [valid, setValid] = useState(true);
  const [workerId, setWorkerId] = useState(null);
  const [connection, setConnection] = useState(null);
  const [result, setResult] = useState(null);
  const [currentFile, setCurrentFile] = useState(null);

  const close = async (e) => {
    e.preventDefault();
    await state.projects.close(workerId);
    setLocation('/');
  }

  const Close = (props) =>  {
    return <a href="#" onclick={close}>Close [X]</a>
  }

  useEffect(async () => {
    state.history.clear();

    let workerId;

    // During development this will spawn a new project process
    // when live reload is enabled so we should close any existng
    // worker process.
    const unload = async () => {
      if (workerId) {
        await state.projects.close(workerId);
      }
    }
    window.addEventListener('unload', unload);

    try {
      const project = await state.projects.find(params.id);
      if (project) {
        workerId = await state.projects.open(project.path);
        setWorkerId(workerId);
        setResult(project);

        // FIXME: add a timeout for this poll!
        let id = null;
        const poll = async () => {
          if (workerId) {
            const info = await state.projects.status(workerId);
            if (info) {
              setConnection(info);
              clearInterval(id);
            }
          }
        }
        id = setInterval(poll, 500);
      } else {
        setValid(false);
      }
    } catch(e) {
      state.flash.error(e);
    }

    return () => {
      window.removeEventListener('unload', unload);
    }
  }, []);

  if (!result && valid) {
    return null;
  } else if (!result && !valid) {
    return <div>
      <Close />
      <p>Project not found</p>
    </div>;
  } else if (result && valid && connection) {
    const webdav = createClient(connection.url + '/-/webdav');
    const onOpenFile = (item) => {
      setCurrentFile(item);
    }

        //<p>Id: {result.id}</p>
        //<p>Path: {result.path}</p>

    return <div class="project-editor">
      <header class="no-select">
        <Close />
      </header>
      <section>
        <FileManager
          webdav={webdav}
          onOpenFile={onOpenFile} />
        <FileEditor
          webdav={webdav}
          file={currentFile} />
        <WebsitePreview url={connection.url} />
      </section>
    </div>;
  }
}
