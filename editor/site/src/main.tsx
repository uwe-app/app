import { h, Fragment, render, createContext } from 'preact';
import {Route} from 'wouter';

import * as _ from './preamble';
import Launch from './Launch';
import Flash from './Flash';
import {CreateProject} from './CreateProject';
import {value, State} from './State';

/*
window.addEventListener('contextmenu', (e) => {
  console.log('context menu event');
  e.preventDefault();
});
*/

render(<State.Provider value={value}>
    <Flash />
    <Route path="/">
      <Launch />
    </Route>
    <Route path="/create">
      <CreateProject />
    </Route>
</State.Provider>, document.body);
