import {h, Fragment} from 'preact';
import Flash from './Flash';
import {ProjectsList, OpenProject} from './Projects';

export default function App(props) {
  return <>
    <Flash />
    <ProjectsList />
    <OpenProject />
  </>
}
