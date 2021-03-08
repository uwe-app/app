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
    console.log("Got projects fetch result", list);
    this.list = list;
  }

  async add(entry) {
    const result = await window.rpc.call('project.add', entry);
    console.log('Project add result', result);
  }

  async remove(item) {
    console.log("Remove ", item);
    const result = await window.rpc.call('project.remove', item.entry);
    console.log('Project remove result', result);
    await this.fetch();
  }
}

class Window {
  constructor() {
    this.fullscreen = false
    makeAutoObservable(this)
  }

  async toggle() {
    await window.rpc.call('window.set_fullscreen', !this.fullscreen);
    this.fullscreen = !this.fullscreen
  }
}

const value = observable({
  window: new Window(),
  //projects: new Projects([{project: 'fubar'}]),
  projects: new Projects(),
});

/*
const toggleFullScreen = action(async (state) => {
  await window.rpc.call('window.set_fullscreen', !state.window.fullscreen);
  state.window.fullscreen = !state.window.fullscreen;

  console.log('Equal', state === value)
});
*/

const State = createContext(value);
export {value, State};
