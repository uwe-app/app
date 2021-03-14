import {h, createRef} from 'preact';
import {useEffect, useState, useContext} from 'preact/hooks';
import {State} from './State';

const getLocation = (base, value) => {
  const path = value.replace(/^\/+/, '');
  return `${base.protocol}//${base.host}/${path}`;
}

const Header = ({ base, onChange, onRefresh }) => {
  const state = useContext(State);
  const [value, setValue] = useState(null);
  const [address, setAddress] = useState(base);

  const setLocation = (path) => {
    const src = getLocation(base, path);
    const url = new URL(src);
    console.log('Set location', path);
    //const url = new URL(path);
    setAddress(url);
    onChange(path);
  }

  const onHome = (e) => {
    e.preventDefault();
    setValue('/');
    setLocation('/?history=1&t=' + Date.now());
  }

  const onReload = (e) => {
    e.preventDefault();
    onRefresh(value);
  }

  const onBack = (e) => {
    e.preventDefault();
    const path = state.history.back();
    if (path) {
        setValue(path);
        setLocation(path + '?history=1&t=' + Date.now());
    }
  }

  const onForward = (e) => {
    e.preventDefault();
    const path = state.history.forward();
    if (path) {
        setValue(path);
        setLocation(path + '?history=1&t=' + Date.now());
    }
  }

  const onSubmit = (e) => {
    e.preventDefault();
    setLocation(value);
  }

  useEffect(() => {
    const onMessage = (e) => {
      const location = new URL(e.data);
      if (value !== location.pathname) {
        setValue(location.pathname);
        setAddress(location.pathname);
        state.history.push(location.pathname);
      }
    }
    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, []);

  const canHome = address.pathname != '/';

  //NOTE: If the button(s) are in the <form> it prevents onSubmit firing!!!
  return <header>
    <form onsubmit={onSubmit}>
      <input
        class="address"
        type="text"
        onChange={(e) => setValue(e.target.value)} value={value} />
    </form>
    <div>
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
    </div>
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

  const onChange = (value) => {
    console.log('Changing iframe src', value);
    setSource(getLocation(base, value));
  }

  const onRefresh = (value) => {
    const url = getLocation(base, value);
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
