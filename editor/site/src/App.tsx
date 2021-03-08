import {h} from 'preact';
import ChooseFolder from './ChooseFolder';
//import ToggleFullScreen from './ToggleFullScreen';
//import DropZone from './DropZone';

import {ProjectsList} from './Projects';

export default function App(props) {
  return <div>
    <ChooseFolder title="Choose a project" />
    <ProjectsList />
  </div>
}
