+++
title = "04. New project"
+++

In the *launch view* if a user selects the *New Project* option they are presented with a view allowing blueprint selection and controls for project fields.

When the user selects *Create Project* and it is successful they will move to the *Project editor* view. Form validation or project creation errors can be shown inline.

<div class="wireframe flex padded sml rhythm">
  <span>Main tool bar w/ Logo</span>
</div>

<div class="wireframe flex column padded center">

  <div class="flex spacer-x center rhythm">
    <a href="#">&lt; Back</a>
  </div>

  <div class="flex column spacer-x rhythm">
    <small>Enter a name for the project:</small>
    <input type="text" placeholder="Project name" />
  </div>

  <div class="flex spacer-x">
    <small>Create project in: /Users/johndoe</small>
    <button>Change folder</button>
  </div>

  <ul>
    <li class="flex">
      <input checked id="basic" name="blueprint" type="radio"></input>
      <label for="basic">Basic website</label>
    </li>
    <li class="flex">
      <input id="blog" name="blueprint" type="radio"></input>
      <label for="blog">Blog</label>
    </li>
    <li class="flex">
      <input id="deck" name="blueprint" type="radio"></input>
      <label for="deck">Deck</label>
    </li>
  </ul>

  <div class="flex spacer-x center">
    <button>Create project</button>
  </div>

</div>
