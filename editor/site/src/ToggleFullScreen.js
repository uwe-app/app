export default function ToggleFullScreen(props) {

  const click = async () => {
    const res = window.rpc.call('window.set_fullscreen', !this.fullscreen);
    this.fullscreen = !this.fullscreen;
    await res;
  }

  return <button onclick={click}>Toggle Fullscreen</button>
}
