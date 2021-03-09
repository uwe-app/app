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
