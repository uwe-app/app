//window.process = {env: {NODE_ENV: 'production'}};
import { h, Fragment, render, createContext } from 'preact';
import * as _ from './preamble';
import App from './App';
//import {State} from './State';

//console.log(State);

//

//const State = createContext('state');

//console.log(State.Provider);

//const State = createContext('state');

/*
const State = createContext('state');
*/

/*
const state = observable({
  window: {
    fullscreen: false
  },
});
*/


render(<App />, document.body);
