export default function DropZone(props) {

  function dragOver(ev) {
    // Prevent default behavior (Prevent file from being opened)
    ev.preventDefault();
  }

  function drop(ev) {
    console.log('File(s) dropped', ev.dataTransfer.files);
    console.log('File(s) dropped', ev.dataTransfer.files.length);
    // Prevent default behavior (Prevent file from being opened)
    ev.preventDefault();

    if (ev.dataTransfer.files) {
      for (var i = 0; i < ev.dataTransfer.files.length; i++) {
        console.log(ev.dataTransfer.files[i].name);
      }
    }
  }

  return (
    <div id="drop_zone" ondrop={drop} ondragover={dragOver}>
      <p>Drag one or more files to this Drop Zone ...</p>
    </div>
  );
}
