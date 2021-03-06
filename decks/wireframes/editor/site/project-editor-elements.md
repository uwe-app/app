+++
title = "07. Project editor (elements)"
+++

Below the toolbar for the *Project Editor* are the main elements a *File Manager*, *File Editor* and *Website Preview* dividing the space into three columns.

Each column has an action bar at the top and a status bar beneath.

We should design controls to minimize/maximize the file manager and file editor so it is possible to switch to a two column view (editor and preview) or just focus on the website preview.

<div class="flex">
  <div class="wireframe flex column" style="flex: 0; flex-basis: 240px;">
    <div>File manager actions</div>
    <div class="wireframe">File manager content</div>
    <div>File manager status (eg: current directory)</div>
  </div>
  <div class="flex space-evenly fill" style="flex: 1;">
    <div class="wireframe flex column fill">
      <div>File selection control</div>
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

