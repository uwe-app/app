import {h, createRef} from 'preact';
import {useEffect, useState, useContext} from 'preact/hooks';
import {State} from './State';

const Header = ({ base, onChange, onRefresh }) => {
  const state = useContext(State);

  const [address, setAddress] = useState(null);

  /*
  let [history, setHistory] = useState(() => []);
  const [cursor, setCursor] = useState(() => -1);
  const pushHistory = (path) => {
    console.log('pushHistory', history.length);
    console.log('pushHistory', cursor);
    // Truncate history when navigating whilst the cursor
    // points to a history entry that is not the last one
    if (cursor > -1 && cursor < history.length - 1) {
      console.log('Truncating history...', cursor)
      history = history.slice(0, cursor - 1);
    }

    history.push(path);
    setCursor(history.length - 1);
  }
  */

  const onHome = (e) => {
    e.preventDefault();
    setAddress('/');
    onChange('/?history=1&t=' + Date.now());
  }

  const onReload = (e) => {
    e.preventDefault();
    onRefresh(address);
  }

  const onBack = (e) => {
    e.preventDefault();
    const path = state.history.back();
    if (path) {
        setAddress(path);
        onChange(path + '?history=1&t=' + Date.now());
    }
  }

  const onForward = (e) => {
    e.preventDefault();
    const path = state.history.forward();
    if (path) {
        setAddress(path);
        onChange(path + '?history=1&t=' + Date.now());
    }
  }

  const onSubmit = (e) => {
    e.preventDefault();
    //alert('Got form submission...');
    //console.log('Submit with address: ', address);
    onChange(address);
  }

  useEffect(() => {
    const onMessage = (e) => {
      const location = new URL(e.data);
      if (address !== location.pathname) {
        setAddress(location.pathname);
        state.history.push(location.pathname);
      }
    }
    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, []);

  const canHome = address != '/';

  //FIXME: the button(s) in the form prevents onSubmit firing!!!

  return <header>
    <form onsubmit={onSubmit}>
      <input
        class="address"
        type="text"
        onChange={(e) => setAddress(e.target.value)} value={address} />
      <button
        disabled={!canHome}
        onclick={onHome}>H</button>
      <button
        onclick={onReload}>O</button>
      <button
        disabled={!state.history.canBack()}
        onclick={onBack}>&lt;</button>
      <button
        disabled={!state.history.canForward()}
        onclick={onForward}>&gt;</button>
    </form>
  </header>;
}

const Content = ({ src }) => {
  return <iframe
    id="preview"
    class="preview content"
    src={src}
    frameborder="0"
    sandbox="allow-scripts allow-forms"
    />;
}

const Footer = () => {
  const [dimensions, setDimensions] = useState(null);

  useEffect(() => {
    const onResize = (e) => {
      const el = document.querySelector('iframe.preview');
      if (el) {
        setDimensions({width: el.offsetWidth, height: el.offsetHeight});
      }
    }
    window.addEventListener('resize', onResize);
    onResize();
    return () => window.removeEventListener('resize', onResize);
  }, []);

  if (dimensions) {
    return <footer class="no-select">
      <small>{dimensions.width}x{dimensions.height}</small>
    </footer>;
  }
  return null;
}

export default function WebsitePreview({ url }) {
  const [src, setSource] = useState(url);
  const base = new URL(url);

  const getLocation = (value) => {
    const path = value.replace(/^\/+/, '');
    return `${base.protocol}//${base.host}/${path}`;
  }

  const onChange = (value) => {
    console.log('Changing iframe src', value);
    setSource(getLocation(value));
  }

  const onRefresh = (value) => {
    const url = getLocation(value);
    const src = `${url}?history=1&t=` + Date.now();
    setSource(src);
  }

  return <div class="website-preview">
    <Header
      base={base}
      onChange={onChange}
      onRefresh={onRefresh} />
    <Content src={src} />
    <Footer />
  </div>;
}
