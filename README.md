# Hyper Text

Fast, opinionated static site generation combining [commonmark][], [handlebars][] with [mdbook][]; see [site](/site) for an example.

```
hypertext
```

Process all files in `site` and write the result to `build` using these rules:

* Parse `.html` and `.hbs` as HTML templates.
* Parse `.md` files as Markdown templates.
* For each parsed template for `hypertext.hbs` in the current directory and parents; if `hypertext.hbs` is found use it as a master template passing the file `content` (see [hypertext.hbs](/site/hypertext.hbs)).
* Document title is inferred from the file name or parent directory.
* If a parse template has a sibling `.toml` file it is used to define document meta data such as the `title` overriding the inferred title.
* If a directory contains a `book.toml` file build using [mdbook][].
* If the directory matches `site/theme` treat as a global theme for [mdbook][].
* Skip any files matched by ignore patterns (`--ignore`).
* Copy all other files.

## Help

See all options with `hypertext --help`.

[commonmark]: https://commonmark.org/
[handlebars]: https://handlebarsjs.com/
[mdbook]: https://rust-lang.github.io/mdBook/
