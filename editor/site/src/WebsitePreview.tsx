import {h, createRef} from 'preact';
import {useEffect, useState} from 'preact/hooks';

const Header = ({ address, onChange }) => {
  const [value, setValue] = useState(address);

  const onSubmit = (e) => {
    e.preventDefault();
    onChange(value);
  }

  useEffect(() => {
    const onMessage = (e) => {
      const location = new URL(e.data);
      setValue(location.pathname);
    }
    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, []);

  return <header>
    <form onsubmit={onSubmit}>
      <input
        class="address"
        type="text"
        onChange={(e) => setValue(e.target.value)} value={value} />
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
  const [source, setSource] = useState(url);
  const [address, setAddress] = useState("/");
  const [dimensions, setDimensions] = useState(null);
  const base = new URL(url);

  const onAddressChange = (value) => {
    const path = value.replace(/^\/+/, '');
    const src = `${base.protocol}//${base.host}/${path}`;
    setSource(src);
  }

  const onSize = (width, height) => {
    setDimensions({width, height});
  }

  return <div class="website-preview">
    <Header onChange={onAddressChange} address={address} />
    <Content url={source} onSize={onSize} />
    <Footer dimensions={dimensions} />
  </div>;
}
