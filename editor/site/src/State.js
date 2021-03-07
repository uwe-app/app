import {createContext} from 'preact';
import {observable, action, makeAutoObservable} from 'mobx';

class Window {
  constructor() {
    this.fullscreen = false
    makeAutoObservable(this)
  }

  async toggle() {
    await window.rpc.call('window.set_fullscreen', !this.fullscreen);
    this.fullscreen = !this.fullscreen
  }
}

const value = observable({
  window: new Window(),
});

/*
const toggleFullScreen = action(async (state) => {
  await window.rpc.call('window.set_fullscreen', !state.window.fullscreen);
  state.window.fullscreen = !state.window.fullscreen;

  console.log('Equal', state === value)
});
*/

const State = createContext(value);
export {value, State};

