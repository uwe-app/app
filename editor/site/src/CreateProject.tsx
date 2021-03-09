import {h, Fragment} from 'preact';
import {useContext, useState} from 'preact/hooks';
import {State} from './State'
import {Link} from 'wouter';

function CloseCreate(props) {
  return <Link href="/">
    <a href="#">Close [X]</a>
  </Link>;
}

function CreateProject(props) {
  const state = useContext(State);

  const [name, setName] = useState('');
  const [target, setTarget] = useState(state.preferences.project.target);
  const [source, setSource] = useState('default');

  const submit = async (e) => {
    e.preventDefault();
    if (!name) {
      return state.flash.error('Project name is required.');
    } else if (!target) {
      return state.flash.error('Project folder path is required.');
    }
    const form = {
      name,
      target,
      source,
    }

    try {
      const path = await state.projects.create(form);
      const entry = {path}
      await state.projects.add(entry);
      await state.projects.fetch();
      state.flash.info(`Created project ${path}`);
    } catch (e) {
      state.flash.error(e);
    }
  }

  const chooseFolder = async (e) => {
    e.preventDefault();
    const path = await state.dialog.openFolder("Choose a folder");
    if (path) {
      setTarget(path)
    }
  }

  const select = (e) => {
    setSource(e.target.value);
  }

  return <div>
    <CloseCreate />
    <form onsubmit={submit}>
      <fieldset>
        <label for="project-name">Name for the new project:</label>
        <input
          id="project-name"
          type="text"
          onchange={e => setName(e.target.value.trim())}
          value={name}
          placeholder="Project name" />
      </fieldset>
      <fieldset class="flex space-between">
        <label>{target}</label>
        <button onclick={chooseFolder}>Choose folder</button>
      </fieldset>
      <fieldset>
        <p>Select a blueprint:</p>
        <div>
          <input
            id="default"
            checked={source === 'default'}
            onclick={select}
            name="source"
            type="radio"
            value="default" />
          <label for="default">Website</label>
        </div>
        <div>
          <input
            id="blog"
            checked={source === 'blog'}
            onclick={select}
            name="source"
            type="radio"
            value="blog" />
          <label for="blog">Blog</label>
        </div>
        <div>
          <input
            id="deck"
            checked={source === 'deck'}
            onclick={select}
            name="source"
            type="radio"
            value="deck" />
          <label for="deck">Deck</label>
        </div>
      </fieldset>
      <input type="submit" value="Create Project" />
    </form>
  </div>;
}

export {CreateProject}
