export default function ChooseFolder(props) {

  const click = async () => {
    try {
      const path = await window.rpc.call('folder.open', props.title)
      if (path == undefined) {
        console.log("User did not choose a folder (cancelled)") ;
      } else {
        console.log("User picked a folder", path);
      }
      document.getElementById('folder-result').innerText = path;
    } catch(e) {
      console.error("Got error choosing folder", e);
    }
  }

  return (
    <div>
      <button onclick={click}>Select folder</button>
      <div id="folder-result"></div>
    </div>
  );
}
