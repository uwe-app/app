import { h, Fragment, render, createContext } from 'preact';
import * as _ from './preamble';
import App from './App';
import {value, State} from './State';
render(<State.Provider value={value}><App /></State.Provider>, document.body);
