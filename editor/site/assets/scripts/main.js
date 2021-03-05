const embedded = typeof rpc !== 'undefined';

if (embedded) {
  const tee = false;

  const console_methods = {
    log: console.log,
    info: console.info,
    warn: console.warn,
    error: console.error,
  }

  console.log = function() {
    const args = Array.prototype.slice.call(arguments, 0);
    if (tee && typeof console_methods.log === 'function') {
      console_methods.log.apply(null, args);
    }
    return window.rpc.notify('console.log', ...args)
  }
  console.info = function() {
    const args = Array.prototype.slice.call(arguments, 0);
    if (tee && typeof console_methods.info === 'function') {
      console_methods.info.apply(null, args);
    }
    return window.rpc.notify('console.info', ...args);
  }
  console.warn = function() {
    const args = Array.prototype.slice.call(arguments, 0);
    if (tee && typeof console_methods.warn === 'function') {
      console_methods.warn.apply(null, args);
    }
    return window.rpc.notify('console.warn', ...args);
  }
  console.error = function() {
    const args = Array.prototype.slice.call(arguments, 0);
    if (tee && typeof console_methods.error === 'function') {
      console_methods.error.apply(null, args);
    }
    return window.rpc.notify('console.error', ...args);
  }
}

window.onerror = function(message, filename, lineno, colno, error) {
    if (error != null) {
      console.error(error.stack);
    } else {
      console.error(`${message} ${filename} ${lineno}`);
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
  console.log("Choosing project folder...");
  try {
    const path = await openFolder('Choose a project');
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

/*
console.info('App started (info)');
console.log('App started (log)');
console.warn('App started (warn)');
console.error('App started (error)');
*/
