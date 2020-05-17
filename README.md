# Hyper Text

Super fast, opinionated static site generation combining [commonmark][], [handlebars][] with [mdbook][]; see [site](/site) for an example.

```
hypertext
```

> Process all files in `site` and write the result to `build`.

## Rules

* Parse `.html` and `.hbs` as HTML templates.
* Parse `.md` files as Markdown templates.
* For each parsed template look for the layout file `layout.hbs` in the current directory and parents; if a layout is found use it passing the file `content` (see [layout.hbs](/site/layout.hbs)).
* Document title is inferred from the file name or parent directory (in the case of index files).
* If a parse template has a sibling `.toml` file it is used to define document meta data such as the `title` overriding the inferred title (see [index.toml](/site/index.toml)).
* If a directory contains a `book.toml` file build using [mdbook][] (see [guide](/site/guide)).
* If the directory matches `site/theme` treat as a global theme for [mdbook][] builds, **exclude** theme files.
* Treat destination files as clean URLs wherever possible (see [guide](/site/contact.hbs)).
* Skip any files matched by exclude patterns (`--exclude`).
* Copy all other files.

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
