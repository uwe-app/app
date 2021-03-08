import {h} from 'preact';
import {useContext} from 'preact/hooks';
import {observer} from 'mobx-react';
import {State} from './State'

function OpenProject(props) {
  const state = useContext(State);
  const click = async () => {
    try {
      const path = await state.dialog.openFolder("Choose a project");
      if (path) {
        try {
          state.flash.clear();
          await state.projects.add({ path });
          // Update the projects list
          await state.projects.fetch();
        } catch(e) {
          state.flash.error(e.message);
        }
      }
    } catch(e) {
      state.flash.error(e.toString());
    }
  }
  return <button onclick={click}>Open Project</button>;
}

function ProjectsList(props) {
  const state = useContext(State);

  const fetch = async () => {
    await state.projects.fetch();
  }

  const remove = async (item) => {
    await state.projects.remove(item);
  }

  const List = observer(({ projects }) => {
    if (projects.list.length) {
      return <ul class="projects">
        {
          projects.list.map((item) => {
            return <li>
              <span>{item.entry.path} ({item.status.toString()})</span>
              <a href="#" onclick={() => remove(item)}>Remove</a>
            </li>
          })
        }
      </ul>
    } else {
      return <p>No projects yet</p>;
    }
  });

  // Fetch initial projects list
  fetch();

  return <List projects={state.projects} />;
}

export {ProjectsList, OpenProject}
