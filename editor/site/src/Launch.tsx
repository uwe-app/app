import {h, Fragment} from 'preact';
import {ProjectsList, OpenProject, NewProject} from './Projects';

export default function Launch(props) {
  return <>
    <ProjectsList />
    <div class="actions">
      <OpenProject />
      <NewProject />
    </div>
  </>
}
