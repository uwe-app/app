# Hyper Text

Super fast, opinionated, site generator combining [pulldown-cmark][], [handlebars][] with [mdbook][].

```
hypertext --minify
```

> Process all files in `site` and write the result to `build`.

See [site](/site) for example source files.

## Rules

* Treat `.html` and `.md` files as *documents* to be parsed as templates.
* Use the nearest layout (`layout.hbs`) wherever possible.
* Infer document title from the file name (or parent directory in the case of index files).
* If a *document* has a sibling *configuration* (`.toml` file) use it to define the template data (see [index.toml](/site/index.toml)).
* If a directory contains a `book.toml` file build using [mdbook][] (see [guide](/site/guide)).
* If the directory `site/template/theme` exists use it as a global theme for [mdbook][].
* Skip any files matched by exclude patterns (`--exclude`).
* Exclude hidden files.
* Copy all other files.

## Layout

For each document look for a *layout template* (`layout.hbs`) in the current directory and parents; if a layout is found pass it the file `content` (see [layout.hbs](/site/layout.hbs)) for rendering. If no layout is located render the *document*.

## Partials

Partial templates (`.hbs`)  in the `site/template` directory are automatically registered using a relative path without the file extension. So the file `site/template/header.hbs` can be included with:

```
{{> header}}
```

## Data

Global data is loaded from `layout.toml` before file-specific configuration data is merged. 

### Generated

Generated fields are:

* `filepath` Template file path.
* `content` Rendered document content (layouts only).

### Configure

Fields available to configure processing:

* `title` Document title.
* `standalone` Document is standalone.

When the `standalone` field is boolean `true` the document will skip layout processing, if you assign any other value (including other types) it is ignored. Be aware if you set this field in the top-level `layout.toml` then no layouts would be used due to configuration inheritance.

### Custom

You can add custom data to document *configuration* `.toml` files and access it in the document template. See [blog/index.toml](/site/blog/index.toml) and [blog/index.html](/site/blog/index.html) for an example.

## Snippets

Markdown documents are parsed via handlebars before being rendered to HTML which allows document snippets using the normal handlebars syntax. The referenced templates will also be parsed as markdown so we recommend using the `.md.hbs` file extension to distinguish them; the `.hbs` extension is always removed so you can reference snippets like so:

```markdown
{{> snippet.md}}
```

## Clean URLs

When the `--clean-url` option is given treat destination files as clean URLs wherever possible (see [contact](/site/contact.html)). Does not apply to files generated via [mdbook][].

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
