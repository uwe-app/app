import {h, Fragment, render} from 'preact';
import {useEffect} from 'preact/hooks';
import {Route, useLocation} from 'wouter';
import Launch from './Launch';
import Flash from './Flash';
import {CreateProject} from './CreateProject';
import {boot, value, State} from './State';

import makeCachedMatcher from "wouter/matcher";

export default function App() {

  // Monkey patch the navigation to we always clear flash messages!
  if (typeof history !== "undefined") {
    for (const type of ["pushState", "replaceState"]) {
      const original = history[type];
      history[type] = function () {
        const result = original.apply(this, arguments);

        value.flash.clear();

        return result;
      };
    }
  }

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
