import {h} from 'preact';
import {useEffect, useState, useContext} from 'preact/hooks';
import {createClient, FileStat} from 'webdav/web';
import {State} from './State';

const Header = (props) => {
  return <header>File manager actions</header>;
}

const Content = ({ listing }) => {
  return <div class="content">
    <ul>
      {listing.map((item) => {
        return <li>{item.filename}</li>
      })}
    </ul>
  </div>;
}

const Footer = (props) => {
  return <footer>...</footer>;
}

export default function FileManager({ webdav }) {
  const state = useContext(State);
  const client = createClient(webdav);

  const [listing, setListing] = useState([]);

  useEffect(async () => {
    try {
      const directoryItems = await client.getDirectoryContents("/");
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

  return <div class="file-manager">
    <Header />
    <Content listing={listing} />
    <Footer />
  </div>;
}
