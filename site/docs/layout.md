## Layout

Layouts provide the skeleton structure for your pages; they are resolved using the directory hierarchy so you can easily assign a different layout for folders.

For each page (`.md` and `.html`) find a layout template (`layout.hbs`) in the current directory and parents; if a layout is found pass it the file `template` for rendering. If no layout is located render the *page*.

If a page has been marked [standalone](/docs/standalone/) no layout is applied.

