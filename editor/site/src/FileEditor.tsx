import {h} from 'preact';

const Header = (props) => {
  return <header>File editor actions</header>;
}

const Content = (props) => {
  return <div class="content">
    <span>File editor area</span>
  </div>;
}

const Footer = (props) => {
  return <footer>...</footer>;
}

export default function FileEditor(props) {
  return <div class="file-editor">
    <Header />
    <Content />
    <Footer />
  </div>;
}
