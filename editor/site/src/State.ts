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

class Flash {
  message: FlashMessage = {};

  constructor() {
    makeAutoObservable(this);
  }

  clear() {
    this.message = {};
  }

  error(text: string) {
    this.message = { text, className: 'error' }
  }
}

const value = {
  window: new Window(),
  projects: new Projects(),
  dialog: new Dialog(),
  flash: new Flash(),
};

const State = createContext(value);
export {value, State};
