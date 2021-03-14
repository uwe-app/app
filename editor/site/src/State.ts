import {createContext} from 'preact';
import {observable, action, makeAutoObservable} from 'mobx';

enum ProjectStatus {
  Missing,
  Error,
  Ok,
}

interface ProjectEntry {
  path: string,
}

interface ProjectListItem {
  entry: ProjectEntry,
  status: ProjectStatus,
}

class History {
  urls: string[];
  cursor: integer;

  constructor() {
    this.clear();
  }

  clear() {
    this.urls = [];
    this.cursor = -1;
  }

  push(url) {
    if (this.cursor > -1 && this.cursor < this.urls.length - 1) {
      this.urls = this.urls.slice(0, this.cursor + 1);
    }
    this.urls.push(url);
    this.cursor = this.urls.length - 1;
  }

  back() {
    // Skip the first history item (home === /)
    if (this.cursor > 0) {
      this.cursor--;
      return this.urls[this.cursor]
    }
  }

  forward() {
    if (this.urls.length > 1 && this.cursor < this.urls.length - 1) {
      this.cursor++;
      return this.urls[this.cursor];
    }
  }

  canBack () {
    return this.urls.length > 1 && this.cursor > 0;
  }

  canForward() {
    return this.urls.length > 1 && this.cursor > -1 && this.cursor < this.urls.length - 1;
  }

}

class Projects {
  list: ProjectListItem[];

  constructor(entries: ProjectListItem[]) {
    this.list = entries || [];
    makeAutoObservable(this)
  }

  async fetch() {
    const list = await window.rpc.call('project.list');
    this.list = list;
  }

  async create(request) {
    return await window.rpc.call('project.create', request);
  }

  async add(entry) {
    return await window.rpc.call('project.add', entry);
  }

  async remove(item) {
    const result = await window.rpc.call('project.remove', item.entry);
    await this.fetch();
  }

  async find(id) {
    return await window.rpc.call('project.find', id);
  }

  async open(path) {
    return await window.rpc.call('project.open', path);
  }

  async close(workerId) {
    return await window.rpc.notify('project.close', workerId);
  }

  async status(workerId) {
    return await window.rpc.call('project.status', workerId);
  }
}

class Window {
  constructor() {
    this.fullscreen = false;
    makeAutoObservable(this);
  }

  async toggle() {
    await window.rpc.call('window.set_fullscreen', !this.fullscreen);
    this.fullscreen = !this.fullscreen
  }
}

class Dialog {
  async openFolder(title: string) {
    return await window.rpc.call('folder.open', title);
  }
}

interface FlashMessage {
  text: string,
  className: string,
}

interface RpcError {
  message: string,
  data?: string,
}

class Flash {
  message: FlashMessage = {};

  constructor() {
    makeAutoObservable(this);
  }

  clear() {
    this.message = {};
  }

  info(text: string) {
    this.message = { text, className: 'info' }
  }

  error(err: string | RpcError) {
    if (typeof err === 'string') {
      this.message = { text: err, className: 'error' }
    } else {
      let text = err.message;
      if (err.data) {
        text = err.data;
      }
      this.message = { text, className: 'error' }
    }
  }
}

interface ProjectPreferences {
  target?: string,
}

interface Preferences {
  project: ProjectPreferences,
}

const value = {
  booted: false,
  window: new Window(),
  projects: new Projects(),
  dialog: new Dialog(),
  flash: new Flash(),
  preferences: {},
  history: new History(),
};

// Load initial state information for the editor.
const boot = async () => {
  const result = await window.rpc.call('app.boot');
  value.projects.list = result.projects;
  value.preferences = result.preferences;
  value.booted = true;
}

const State = createContext(value);
export {boot, value, State};
