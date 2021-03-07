import {createContext} from 'preact';
import {observable, action} from 'mobx';

const state = {
  window: {
    fullscreen: false
  },
};

const toggleFullScreen = action(async (state) => {
  await window.rpc.call('window.set_fullscreen', !state.window.fullscreen);
  state.window.fullscreen = !state.window.fullscreen;
});

const State = createContext(observable(state));
export {State, toggleFullScreen};

