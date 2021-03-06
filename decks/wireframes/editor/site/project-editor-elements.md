<div>
  <strong>06. Project editor (elements)</strong>
</div>

Below the toolbar for the *Project Editor* are the main elements a file manager, file editor and website preview dividing the space into three columns.

Each column has an action bar at the top and a status bar beneath.

We should design in controls to minimize/maximize the file manager and file editor so it is possible to switch to a two column view (file editor/website preview) or just focus on the website preview.

<div class="flex">
  <div class="wireframe flex column" style="flex: 0; flex-basis: 240px;">
    <div>File manager actions</div>
    <div class="wireframe">File manager content</div>
    <div>File manager status (eg: current directory)</div>
  </div>
  <div class="flex space-evenly fill" style="flex: 1;">
    <div class="wireframe flex column fill">
      <div>File editor tabs</div>
      <div class="wireframe">File editor content</div>
      <div>File editor status (eg: Open file name and file size)</div>
    </div>
    <div class="wireframe flex column fill">
      <div>Address bar to manually navigate</div>
      <div class="wireframe">Website preview</div>
      <div>Mobile/tablet/desktop preview actions</div>
    </div>
  </div>
</div>

