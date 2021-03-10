import {h, Fragment, render} from 'preact';
import {useEffect} from 'preact/hooks';
import {Route, useLocation} from 'wouter';
import Launch from './Launch';
import Flash from './Flash';
import {CreateProject} from './CreateProject';
import {boot, value, State} from './State';

import makeCachedMatcher from "wouter/matcher";

export default function App() {

  const [location, setLocation] = useLocation();

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

  // Look an initial search query and rewrite the location
  // so we fulfill the /project/:id route match.
  //
  // This is required because we want to launch with a query
  // string when a project path is passed on the command line
  // so we only need a single `index.html` file. But `wouter`
  // does not currently support `document.location.search` so
  // we rewrite to a route it can handle.
  //
  // Internally when we want to show a project view we should use
  // a `/project/:id` link.
  useEffect(async () => {
    if (document.location.search) {
      const project = new URLSearchParams(
          window.location.search).get('project');
      if (project) {
        const dest = `/project/${project}`;
        setLocation(dest);
      }
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
    <Route path="/project/:id">
      {(params) => <div>Project, {params.id}!</div>}
    </Route>
  </State.Provider>;
}
