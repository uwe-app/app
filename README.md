# Hyper Text

Fast, opinionated static site generation combining HTML, [commonmark][] with [handlebars][] and [mdbook][] support.

See [site](/site) for example files.

```
hypertext
```

Process all files in `site` and write the result to `build` using these rules:

* Parse `.html` and `.hbs` as HTML templates.
* Parse `.md` files as Markdown templates.
* For each parsed template walk parent directories looking for `hypertext.hbs`.
    If `hypertext.hbs` is found use it as a master template passing the file `content` (see [hypertext.hbs](/site/hypertext.hbs)).
* If a directory contains a `book.toml` file build using [mdbook][].
* If the directory matches `site/theme` treat as a global theme for [mdbook][].
* Skip any files matched by ignore patterns (`--ignore`).
* Copy all other files.

## Help

See all options with `hypertext --help`.

[commonmark]: https://commonmark.org/
[handlebars]: https://handlebarsjs.com/
[mdbook]: https://rust-lang.github.io/mdBook/
