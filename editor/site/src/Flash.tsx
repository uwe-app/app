import {h} from 'preact';
import {useContext} from 'preact/hooks';
import {observer} from 'mobx-react';
import {State} from './State'

export default function Flash(props) {
  const state = useContext(State);

  const dismiss = () => {
    state.flash.clear();
  }

  const Message = observer(({ flash }) => {
    if (flash.message.text) {
      return <div class="flash">
        <div class={flash.message.className}>
          <div><a href="#" onclick={dismiss}>Dismiss</a></div>
          <p>{flash.message.text}</p>
        </div>
      </div>
    }
    return null;
  });

  return <Message flash={state.flash} />
}
