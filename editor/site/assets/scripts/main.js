const isEmbedded = typeof rpc !== 'undefined';

window.onerror = function(message, filename, lineno, colno, error) {
    if (error != null) {
      alert(error.stack);
    } else {
      alert(`${message} ${filename} ${lineno}`);
    }
};

function toggleFullScreen() {
  const res = window.rpc.call('window.set_fullscreen', !this.fullscreen);
  this.fullscreen = !this.fullscreen;
  return res;
}

function openFolder(title) {
  return window.rpc.call('folder.open', title);
}

/*
function openProject(path) {
  return window.rpc.call('project.open', [path]);
}
*/

async function chooseProject() {
  const path = await openFolder('Choose a project');
  alert('Folder path ' + path);
}

//console.info('App started...');
//console.log('App started...');
//console.warn('App started...');
//console.error('App started...');
