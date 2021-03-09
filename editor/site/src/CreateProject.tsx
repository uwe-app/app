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
  const [base, setBase] = useState(state.preferences.base);
  const [blueprint, setBlueprint] = useState('default');

  const submit = async (e) => {
    e.preventDefault();
    if (!name) {
      return state.flash.error('Project name is required.');
    } else if (!base) {
      return state.flash.error('Project folder path is required.');
    }
    const form = {
      name,
      base,
      blueprint,
    }
    console.log("Request", JSON.stringify(form));
  }

  const chooseFolder = async (e) => {
    e.preventDefault();
    const path = await state.dialog.openFolder("Choose a folder");
    if (path) {
      setBase(path)
    }
  }

  const select = (e) => {
    setBlueprint(e.target.value);
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
        <label id="project-base">{base}</label>
        <button onclick={chooseFolder}>Choose folder</button>
      </fieldset>
      <fieldset>
        <p>Select a blueprint:</p>
        <div>
          <input
            id="default"
            checked={blueprint === 'default'}
            onclick={select}
            name="blueprint"
            type="radio"
            value="default" />
          <label for="default">Website</label>
        </div>
        <div>
          <input
            id="blog"
            checked={blueprint === 'blog'}
            onclick={select}
            name="blueprint"
            type="radio"
            value="blog" />
          <label for="blog">Blog</label>
        </div>
        <div>
          <input
            id="deck"
            checked={blueprint === 'deck'}
            onclick={select}
            name="blueprint"
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
