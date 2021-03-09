import {h, Fragment, render} from 'preact';
import {useEffect} from 'preact/hooks';
import {Route} from 'wouter';
import Launch from './Launch';
import Flash from './Flash';
import {CreateProject} from './CreateProject';
import {boot, value, State} from './State';

export default function App() {

  // Load initial app state.
  useEffect(async () => {
    try {
      await boot();
    } catch(e) {
      value.flash.error(e);
    }
  }, []);

  return <State.Provider value={value}>
    <Flash />
    <Route path="/">
      <Launch />
    </Route>
    <Route path="/create">
      <CreateProject />
    </Route>
  </State.Provider>;
}
