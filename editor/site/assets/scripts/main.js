window.onerror = function(message, filename, lineno, colno, error) {
    if (error != null) {
      console.error(error.stack);
    } else {
      console.error(`${message} ${filename} ${lineno}`);
    }
};

function RpcProxy() {

  this._result = function(id, result) {
    console.log("RpcProxy got result to resolve promise", id, result);
    window.external.rpc._result(id, result);
  }

  this._error = function(id, error) {
    console.log("RpcProxy got error to reject promise", id, error);
    window.external.rpc._error(id, error);
  }

  this.call = function() {
    let args = Array.prototype.slice.call(arguments)
    console.log("RpcProxy call: ", args);
    window.external.rpc.call.apply(window.external, args)
  }
}

window.rpc = new RpcProxy();

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
  console.log("Choosing project folder...");
  try {
    const path = await openFolder('Choose a project');
    if (path == undefined) {
      console.log("User did not choose a folder (cancelled)") ;
    } else {
      console.log("User picked a folder", path);
    }
  } catch(e) {
    console.error("Got error choosing folder", e);
  }
}

//console.info('App started...');
//console.log('App started...');
//console.warn('App started...');
//console.error('App started...');
