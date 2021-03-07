import {h} from 'preact';
import {useContext} from 'preact/hooks';
import {State, toggleFullScreen} from './State'

export default function ToggleFullScreen(props) {
  const state = useContext(State);
  const click = async () => {
    await toggleFullScreen(state);
  }
  return <button onclick={click}>Toggle Fullscreen</button>
}
