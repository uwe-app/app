import ChooseFolder from './ChooseFolder';
import ToggleFullScreen from './ToggleFullScreen';
import DropZone from './DropZone';

export default function App(props) {
  return <div>
    <ChooseFolder title="Choose a project" />
    <ToggleFullScreen />
    <DropZone />
  </div>
}
