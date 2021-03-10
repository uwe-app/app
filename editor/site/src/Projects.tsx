import {h} from 'preact';
import {useContext} from 'preact/hooks';
import {observer} from 'mobx-react-lite';
import {State} from './State'
import {Link} from 'wouter';

function NewProject(props) {
  return <Link href="/create">
    <button>New Project</button>
  </Link>;
}

function OpenProject(props) {
  const state = useContext(State);
  const click = async (e) => {
    e.preventDefault();

    try {
      const path = await state.dialog.openFolder("Choose a project");
      if (path) {
        try {
          state.flash.clear();
          await state.projects.add({ path });
          // Update the projects list
          await state.projects.fetch();
        } catch(e) {
          state.flash.error(e);
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

  const remove = async (e, item) => {
    e.preventDefault();
    await state.projects.remove(item);
  }

  const List = observer(({ projects }) => {
    if (projects.list.length) {
      return <ul class="projects">
        {
          projects.list.map((item) => {
            return <li>
              <Link href={'/project/' + item.entry.id}>
                <span>{item.entry.path} ({item.status.toString()})</span>
              </Link>
              <a href="#" onclick={(e) => remove(e, item)}>Remove</a>
            </li>
          })
        }
      </ul>
    } else {
      return <p>No projects yet</p>;
    }
  });

  return <List projects={state.projects} />;
}

export {ProjectsList, OpenProject, NewProject}
