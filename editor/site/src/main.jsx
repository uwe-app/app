import { h, Fragment, render, createContext } from 'preact';
import * as _ from './preamble';
import App from './App';
import {value, State} from './State';

/*
window.addEventListener('contextmenu', (e) => {
  console.log('context menu event');
  e.preventDefault();
});
*/

render(<State.Provider value={value}><App /></State.Provider>, document.body);
