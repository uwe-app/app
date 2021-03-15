import {h} from 'preact';
import {useEffect, useState, useContext} from 'preact/hooks';
import {FileStat} from 'webdav/web';
import {State} from './State';
import humanFileSize from './utils/file-size';

const Header = ({ ancestors, onSelect }) => {

  const onChange = (e) => {
    e.preventDefault();
    onSelect(e.target.value);
  }

  return <header>
    <select onChange={onChange}>
      {ancestors.map((item, i) => {
        const last = i === ancestors.length - 1;
        return <option disabled={last} selected={last} value={item.filename}>{item.basename}</option>
      })}
    </select>
  </header>;
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
        const icon = item.type === 'directory' ? <span>D</span> : <span>F</span>;
        return <li ondoubleclick={(e) => onDoubleClick(e, item)}>{icon} {item.basename}</li>
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
      <small>{listing.length} item(s) ({humanFileSize(size)})</small>
    </footer>;
  }
  return null;
}

export default function FileManager({ webdav, onOpenFile }) {
  const state = useContext(State);
  const [listing, setListing] = useState([]);
  const [directory, setDirectory] = useState(null);
  const [ancestors, setAncestors] = useState(['/']);

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

      if (path === '/') {
        setAncestors([{filename: '/', basename: '/'}])
      } else {
        const parts = path.split('/');
        let tmp = []
        const ancestors = parts.map((part) => {
          tmp.push(part);
          let basename;
          let filename;
          if (part === '') {
            basename = '/';
            filename = '/';
          } else {
            basename = part;
            filename = tmp.length ? tmp.join('/') : '/';
          }
          return {basename, filename};
        });
        setAncestors(ancestors);
      }
    } catch(e) {
      state.flash.error(e);
    }
  };

  useEffect(async () => {
    onOpenDirectory('/');
  }, []);

  return <div class="file-manager no-select">
    <Header ancestors={ancestors} onSelect={onOpenDirectory} />
    <Content
      listing={listing}
      onOpenFile={onOpenFile}
      onOpenDirectory={onOpenDirectory} />
    <Footer listing={listing} />
  </div>;
}
