# Hyper Text

Super fast, opinionated static site generation combining [pulldown-cmark][], [handlebars][] with [mdbook][]; see [site](/site) for an example.

```
hypertext
```

> Process all files in `site` and write the result to `build`.

## Rules

* Treat `.html` and `.md` files as *documents* to be parsed as templates using a layout.
* Treat `.hbs` files as *templates*; never copy them to `build`.
* Infer document title from the file name or parent directory (in the case of index files).
* If a *document* has a sibling *configuration* (`.toml` file) use it to define the template data (see [index.toml](/site/index.toml)).
* If a directory contains a `book.toml` file build using [mdbook][] (see [guide](/site/guide)).
* If the directory matches `site/theme` treat as a global theme for [mdbook][] builds, **exclude** theme files.
* Treat destination files as clean URLs wherever possible (see [contact](/site/contact.html)).
* Skip any files matched by exclude patterns (`--exclude`).
* Copy all other files.

## Layout

For each document look for a *layout template* (`layout.hbs`) in the current directory and parents; if a layout is found pass it the file `content` (see [layout.hbs](/site/layout.hbs)) for rendering. If no layout is located render the *document*.

## Template Data

Templates are exposed the following fields:

* `filepath` Template file path.
* `title` Document title.

Layout templates have an additional `content` field containing the rendered template content.

## Snippets

Markdown documents are parsed via handlebars before being converted to HTML which allows including other markdown document snippets using the normal handlebars syntax. The referenced templates will also be parsed as markdown so we recommend using the `.md.hbs` file extension to distinguish them from.

The `.hbs` extension is always removed so you can reference snippets like this:

```markdown
{{> snippet.md}}
```

## Bugs

* Due to the `theme` convention the site cannot have a top-level `theme` directory.

## Help

Get short help with `hypertext -h` and see more detail with `hypertext --help`.

## Notes

The files in [site](/site) are an example to demonstrate and test various configurations and are clearly not the best way to structure a site.

Much inspiration lifted from [mdbook][].

## License

See [LICENSE](/LICENSE).

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
