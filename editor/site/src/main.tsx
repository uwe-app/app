import {h, render} from 'preact';
import preamble from './preamble';
import App from './App';

const {embedded} = preamble();

if (embedded) {
  render(<App />, document.querySelector('article'));
} else {
  render(<p>Unsupported environment</p>, document.querySelector('article'));
}
