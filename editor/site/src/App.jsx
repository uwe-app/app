import {h} from 'preact';
import ChooseFolder from './ChooseFolder';
import ToggleFullScreen from './ToggleFullScreen';
import DropZone from './DropZone';


//import State from './State';
//
//console.log('createContext' + preact);
/*
for (z in preact) {
  console.log(z);
}
*/

export default function App(props) {
  return <div>
    <ChooseFolder title="Choose a project" />
    <ToggleFullScreen />
    <DropZone />
  </div>
}
