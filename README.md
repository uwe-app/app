# Hyper Text

Super fast, opinionated static site generation combining [commonmark][], [handlebars][] with [mdbook][]; see [site](/site) for an example.

```
hypertext
```

> Process all files in `site` and write the result to `build`.

## Rules

* Parse `.html` and `.hbs` as HTML *template files*.
* Parse `.md` files as Markdown *template files*.
* For each parsed *template file* look for the *layout file* `layout.hbs` in the current directory and parents; if a layout is found pass it the file `content` (see [layout.hbs](/site/layout.hbs)) for rendering. If no layout is located render the *template file*.
* Infer document title from the file name or parent directory (in the case of index files).
* If a *template file* has a sibling *data file* (`.toml`) use it to define the template data (see [index.toml](/site/index.toml)).
* If a directory contains a `book.toml` file build using [mdbook][] (see [guide](/site/guide)).
* If the directory matches `site/theme` treat as a global theme for [mdbook][] builds, **exclude** theme files.
* Treat destination files as clean URLs wherever possible (see [contact](/site/contact.hbs)).
* Skip any files matched by exclude patterns (`--exclude`).
* Copy all other files.

## Template Data

Templates are exposed the following fields:

* `filepath` Template file path.
* `title` Document title.

Layout templates have an additional `content` field containing the rendered template content.

## Bugs

* Due to the `theme` convention the site cannot have a top-level `theme` directory.
* Cannot use `layout.hbs` as input to a page template.

## Help

Get short help with `hypertext -h` and see more detail with `hypertext --help`.

## Notes

The files in [site](/site) are an example to demonstrate and test various configurations and are clearly not the best way to structure a site.

Much inspiration lifted from [mdbook][].

## License

See [LICENSE](/LICENSE).

[commonmark]: https://commonmark.org/
[handlebars]: https://handlebarsjs.com/
[mdbook]: https://rust-lang.github.io/mdBook/
