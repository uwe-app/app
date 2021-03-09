import { h, render } from 'preact';
import preamble from './preamble';
import App from './App';

preamble();

render(<App />, document.body);
