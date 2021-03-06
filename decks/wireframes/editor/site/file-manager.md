+++
title = "08. File manager"
+++

The *File Manager* allows creating and removing files and folders; it must support the usual shift and ctrl modifiers for multiple selection. A search filter at the top will fuzzy match on all files and update the list to only show matches. The plus and minus buttons beneath the list can be used to create and remove files. Removing files will require a confirmation dialog and creation will need to handle folders as well as files.

<div class="wireframe flex column">
  <div class="flex">
    <input type="search" placeholder="Filter files" style="width: 100%;" />
    <button>Clear</button>
  </div>
  <div>
    <ul>
      <li>assets/</li>
      <li>about.md</li>
      <li>index.md</li>
    </ul>
  </div>
  <div class="flex space-between">
    <button>-</button> 
    <span>1 folder, 2 files</span>
    <button>+</button> 
  </div>
</div>

