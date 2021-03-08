import {h} from 'preact';
import {useContext} from 'preact/hooks';
import {observer} from 'mobx-react';
import {State} from './State'

function ProjectsList(props) {
  const state = useContext(State);

  const fetch = async () => {
    await state.projects.fetch();
  }

  const forget = async (item) => {
    await state.projects.remove(item);
  }

  const List = observer(({ projects }) => {
    return <ul class="projects">
      {
        projects.list.map((item) => {
          return <li>
            <span>{item.entry.path} ({item.status.toString()})</span>
            <a href="#" onclick={() => forget(item)}>Forget</a>
          </li>
        })
      }
    </ul>
  });

  fetch();

  return <List projects={state.projects} />
}

export {ProjectsList}
