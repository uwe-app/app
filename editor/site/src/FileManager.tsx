import {h} from 'preact';
import {useEffect, useState, useContext} from 'preact/hooks';
import {FileStat} from 'webdav/web';
import {State} from './State';

const Header = (props) => {
  return <header>File manager actions</header>;
}

const Content = ({ listing, onOpenFile }) => {

  const onDoubleClick = (e, item) => {
    e.preventDefault();
    if (item.type === 'file') {
      onOpenFile(item);
    }
  };

  return <div class="content">
    <ul>
      {listing.map((item) => {
        return <li ondoubleclick={(e) => onDoubleClick(e, item)}>{item.filename}</li>
      })}
    </ul>
  </div>;
}

const Footer = (props) => {
  return <footer>...</footer>;
}

export default function FileManager({ webdav, onOpenFile }) {
  const state = useContext(State);

  const [listing, setListing] = useState([]);

  useEffect(async () => {
    try {
      const directoryItems = await webdav.getDirectoryContents("/");
      console.log(directoryItems);

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
      //console.log("Got webdav directory items: ", directoryItems);
      setListing(listing);
    } catch(e) {
      state.flash.error(e);
    }
  }, []);

  //console.log('Render with listing', listing);

  return <div class="file-manager no-select">
    <Header />
    <Content listing={listing} onOpenFile={onOpenFile} />
    <Footer />
  </div>;
}
