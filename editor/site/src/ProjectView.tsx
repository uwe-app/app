import {h} from 'preact';
import {useEffect, useContext, useState} from 'preact/hooks';
import {useRoute, useLocation} from 'wouter';
import {Link} from 'wouter';
import {State} from './State'
import FileManager from './FileManager';
import WebsitePreview from './WebsitePreview';

export default function ProjectView() {
  const state = useContext(State);
  const [location, setLocation] = useLocation();

  const [match, params] = useRoute("/project/:id");
  const [valid, setValid] = useState(true);
  const [workerId, setWorkerId] = useState(null);
  const [connection, setConnection] = useState(null);
  const [result, setResult] = useState(null);

  const close = async (e) => {
    e.preventDefault();
    await state.projects.close(workerId);
    setLocation('/');
  }

  const Close = (props) =>  {
    return <a href="#" onclick={close}>Close [X]</a>
  }

  useEffect(async () => {
    try {
      const project = await state.projects.find(params.id);
      if (project) {
        const workerId = await state.projects.open(project.path);
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
  }, []);

  if (!result && valid) {
    return null;
  } else if (!result && !valid) {
    return <div>
      <Close />
      <p>Project not found</p>
    </div>;
  } else if (result && valid && connection) {
    return <div class="project-editor">
      <header>
        <Close />
        <p>Id: {result.id}</p>
        <p>Path: {result.path}</p>
      </header>
      <section>
        <FileManager webdav={connection.url + '/-/webdav'} />
        <WebsitePreview url={connection.url} />
      </section>
    </div>;
  }
}
