import {h} from 'preact';
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
      <input type="text" onChange={(e) => setValue(e.target.value)} value={value} />
    </form>
  </header>;
}

const Content = ({ url }) => {
  return <iframe
    id="preview"
    class="preview content"
    src={url}
    frameborder="0"
    sandbox="allow-scripts allow-forms allow-same-origin"
    />;
}

const Footer = (props) => {
  return <footer>...</footer>;
}

export default function WebsitePreview({ url }) {

  const [source, setSource] = useState(url);
  const [address, setAddress] = useState("/");
  const base = new URL(url);

  const onAddressChange = (value) => {
    const path = value.replace(/^\/+/, '');
    const src = `${base.protocol}//${base.host}/${path}`;
    setSource(src);
  }

  return <div class="website-preview">
    <Header onChange={onAddressChange} address={address} />
    <Content url={source} />
    <Footer />
  </div>;
}
