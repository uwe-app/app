import {h} from 'preact';
import {useEffect, useContext, useState} from 'preact/hooks';
import {useRoute} from 'wouter';
import {Link} from 'wouter';
import {State} from './State'

function Close(props) {
  return <Link href="/">
    <a href="#">Close [X]</a>
  </Link>;
}

export default function ProjectView() {
  const state = useContext(State);

  const [match, params] = useRoute("/project/:id");
  const [valid, setValid] = useState(true);
  const [result, setResult] = useState(null);

  useEffect(async () => {
    try {
      const project = await state.projects.find(params.id);
      if (project) {
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
