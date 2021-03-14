import {h, createRef} from 'preact';
import {useEffect, useState} from 'preact/hooks';

const Header = ({ address, setAddress, pushHistory, onChange, onHome, onBack, onForward }) => {
  const onSubmit = (e) => {
    e.preventDefault();
    onChange(address);
  }

  useEffect(() => {
    const onMessage = (e) => {
      const location = new URL(e.data);
      if (address !== location.pathname) {
        setAddress(location.pathname);
        pushHistory(location.pathname);
      }
    }
    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, []);

  return <header>
    <form onsubmit={onSubmit}>
      <input
        class="address"
        type="text"
        onChange={(e) => setAddress(e.target.value)} value={address} />
      <button onclick={onHome}>H</button>
      <button onclick={onBack}>&lt;</button>
      <button onclick={onForward}>&gt;</button>
    </form>
  </header>;
}

const Content = ({ url, onSize }) => {

  const preview = createRef();

  useEffect(() => {
    const onResize = (e) => {
      const el = document.querySelector('iframe.preview');
      if (el) {
        onSize(el.offsetWidth, el.offsetHeight);
      }
    }
    window.addEventListener('resize', onResize);
    onResize();
    return () => window.removeEventListener('resize', onResize);
  }, []);

  return <iframe
    id="preview"
    ref={preview}
    class="preview content"
    src={url}
    frameborder="0"
    sandbox="allow-scripts allow-forms"
    />;
}

const Footer = ({ dimensions }) => {
  if (dimensions) {
    return <footer class="no-select">
      <small>{dimensions.width}x{dimensions.height}</small>
    </footer>;
  }
  return null;
}

export default function WebsitePreview({ url }) {
  const [address, setAddress] = useState(null);
  const [source, setSource] = useState(url);
  const [dimensions, setDimensions] = useState(null);
  const [history, setHistory] = useState([]);
  const [cursor, setCursor] = useState(-1);

  const base = new URL(url);

  const onAddressChange = (value) => {
    const path = value.replace(/^\/+/, '');
    const src = `${base.protocol}//${base.host}/${path}`;
    setSource(src);
  }

  const pushHistory = (path) => {
    history.push(path);
    setCursor(history.length - 1);
  }

  const onHome = (e) => {
    e.preventDefault();
    const src = `${base.protocol}//${base.host}/?t=` + Date.now();
    setSource(src);
  }

  const onBack = (e) => {
    e.preventDefault();
    // Skip the first history item (home === /)
    if (history.length > 1) {
      let pos = cursor;
      if (pos == -1) {
        pos = history.length - 2;
      } else {
        pos = cursor - 1;
      }

      if (history[pos]) {
        const path = history[pos];
        const src = `${base.protocol}//${base.host}${path}?history=1&t=` + Date.now();
        setSource(src);
        setAddress(path);
        setCursor(pos);
      }
    }
  }

  const onForward = (e) => {
    e.preventDefault();
    if (history.length > 1 && cursor < history.length - 1) {
      let pos = cursor + 1;
      if (history[pos]) {
        const path = history[pos];
        const src = `${base.protocol}//${base.host}${path}?history=1&t=` + Date.now();
        setSource(src);
        setAddress(path);
        setCursor(pos);
      }
    }
  }

  const onSize = (width, height) => {
    setDimensions({width, height});
  }

  return <div class="website-preview">
    <Header
      address={address}
      setAddress={setAddress}
      onChange={onAddressChange}
      onHome={onHome}
      onBack={onBack}
      onForward={onForward}
      pushHistory={pushHistory}
      />
    <Content url={source} onSize={onSize} />
    <Footer dimensions={dimensions} />
  </div>;
}
