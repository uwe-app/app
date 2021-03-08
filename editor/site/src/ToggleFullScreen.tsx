import {h, Fragment} from 'preact';
import {useContext} from 'preact/hooks';
import {observer} from 'mobx-react';
import {State} from './State'

export default function ToggleFullScreen(props) {
  const state = useContext(State);
  const click = async () => {
    await state.window.toggle();
  }

  //const fullscreen = state.window.fullscreen;
  const FullScreen = observer(({ window }) => {
    return <span>Is fullscreen? {window.fullscreen.toString()} </span>
  });

  return <>
    <button onclick={click}>Toggle Fullscreen</button>
    <FullScreen window={state.window} />
  </>;
}
