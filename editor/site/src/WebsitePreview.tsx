import {h} from 'preact';

const Header = (props) => {
  return <header>Website preview address bar</header>;
}

const Content = ({ url }) => {
  return <iframe
      class="preview content"
      src={url}
      frameborder="0"
      sandbox="allow-scripts allow-forms"
      />;
}

const Footer = (props) => {
  return <footer>...</footer>;
}

export default function WebsitePreview({ url }) {
  return <div class="website-preview">
    <Header />
    <Content url={url} />
    <Footer />
  </div>;
}
