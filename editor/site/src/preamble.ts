export default function preamble() {
  const embedded = typeof rpc !== 'undefined';

  if (embedded) {
    const tee = true;

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

    window.onerror = function(message, filename, lineno, colno, error) {
        if (error != null) {
          console.error(error.stack);
        } else {
          console.error(`${message} ${filename} ${lineno}`);
        }
    };
  }
}

