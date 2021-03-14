import {h} from 'preact';
import {useEffect, useState, useContext} from 'preact/hooks';
import {FileStat} from 'webdav/web';
import {State} from './State';

const Header = (props) => {
  return <header>File manager actions</header>;
}

const Content = ({ listing, onOpenFile, onOpenDirectory }) => {

  const onDoubleClick = (e, item) => {
    e.preventDefault();
    if (item.type === 'file') {
      onOpenFile(item);
    } else if (item.type === 'directory') {
      onOpenDirectory(item.filename, item);
    }
  };

  return <div class="content">
    <ul>
      {listing.map((item) => {
        return <li ondoubleclick={(e) => onDoubleClick(e, item)}>{item.basename}</li>
      })}
    </ul>
  </div>;
}

const Footer = ({ listing }) => {

  const size = listing.reduce((acc, item) => {
    acc += item.size;
    return acc;
  }, 0);

  if (listing) {
    return <footer>
      <small>{listing.length} item(s) ({size})</small>
    </footer>;
  }
  return null;
}

export default function FileManager({ webdav, onOpenFile }) {
  const state = useContext(State);
  const [listing, setListing] = useState([]);
  const [directoty, setDirectory] = useState(null);

  const onOpenDirectory = async (path, item) => {
    try {
      const directoryItems = await webdav.getDirectoryContents(path);
      const listing = directoryItems.sort((a, b) => {
        if (a.type === 'directory' && b.type !== 'directory') {
          return -1;
        } else if (b.type === 'directory' && a.type !== 'directory') {
          return 1;
        }
        if (a.filename < b.filename) {
          return -1;
        }
        if (a.filename > b.filename) {
          return 1;
        }
        return 0;
      });
      setListing(listing);
      setDirectory(path, item);
    } catch(e) {
      state.flash.error(e);
    }
  };

  useEffect(async () => {
    onOpenDirectory('/');
  }, []);

  return <div class="file-manager no-select">
    <Header />
    <Content
      listing={listing}
      onOpenFile={onOpenFile}
      onOpenDirectory={onOpenDirectory} />
    <Footer listing={listing} />
  </div>;
}
