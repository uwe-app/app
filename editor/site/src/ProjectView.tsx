import {h} from 'preact';
import {useEffect, useContext, useState} from 'preact/hooks';
import {useRoute, useLocation} from 'wouter';
import {Link} from 'wouter';
import {State} from './State'

export default function ProjectView() {
  const state = useContext(State);
  const [location, setLocation] = useLocation();

  const [match, params] = useRoute("/project/:id");
  const [valid, setValid] = useState(true);
  const [id, setWorkerId] = useState(null);
  const [result, setResult] = useState(null);

  const close = async (e) => {
    e.preventDefault();
    await state.projects.close(id);
    setLocation('/');
  }

  const Close = (props) =>  {
    return <a href="#" onclick={close}>Close [X]</a>
  }

  useEffect(async () => {
    try {
      const project = await state.projects.find(params.id);
      if (project) {
        const id = await state.projects.open(project.path);
        setWorkerId(id);
        setResult(project);
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
  } else if (result && valid) {
    return <div>
      <Close />
      <p>Id: {result.id}</p>
      <p>Path: {result.path}</p>
    </div>;
  }
}
