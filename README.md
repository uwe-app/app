# Hyper Text

Super fast, opinionated, site generator combining [pulldown-cmark][], [handlebars][] and [mdbook][].

```
hypertext --minify
```

> Process all files in `site` and write the result to `build`.

## Rules

* Treat `.html` and `.md` files as *documents* to be parsed as templates.
* Use the nearest layout (`layout.hbs`) wherever possible.
* Infer document title from the file name (or parent directory in the case of index files).
* Load template data from `.toml` files.
* If a directory contains `book.toml` use [mdbook][].
* If the directory `site/template/theme` exists use it as the theme for [mdbook][].
* Exclude hidden files and patterns in `.gitignore`.
* Copy all other files.

See [site](/site) for example source files that demonstrate and test various configurations; they are *not the ideal* way to structure a site.

## Layout

For each document look for a *layout template* (`layout.hbs`) in the current directory and parents; if a layout is found pass it the file `template` (see [layout.hbs](/site/layout.hbs)) for rendering. If no layout is located render the *document*.

## Partials

Partial templates (`.hbs`)  in the `site/template` directory are automatically registered using a relative path without the file extension. So the file `site/template/header.hbs` can be included with:

```
{{> header}}
```

See [header.hbs](/site/template/header.hbs).

## Template Data

Create a file with the same name and `.toml` extension for document-specific configuration; see [index.toml](/site/index.toml) and [about.toml](/site/about.toml).

Data is loaded by searching for `layout.toml` in the current directory and parents, if a file is found it is used before merging with any document-specific configuration file.

### Generated

These fields are **reserved** keywords for template data:

* `context` Helper context information.
* `template` Rendered document content (layouts only).

### Configure

Fields available to configure processing:

* `title` Document title.
* `standalone` Document is standalone.

When the `standalone` field is `true` the document will skip layout processing. Be aware if you set this field in the top-level `layout.toml` then no layouts would be used due to configuration inheritance.

### Custom

You can add custom configuration data and access it in the document template. See [blog/index.toml](/site/blog/index.toml) and [blog/index.html](/site/blog/index.html) for an example.

## Snippets

Markdown documents are parsed via handlebars before being rendered to HTML which allows document snippets using the normal handlebars syntax. The referenced templates will also be parsed as markdown so we recommend using the `.md.hbs` file extension to distinguish them; the `.hbs` extension is always removed so you can reference snippets like so:

```markdown
{{> snippet.md}}
```

## Helpers

Some helpers are available to make life easier.

### JSON

A useful helper to pretty print the JSON data available to your templates.

```handlebars
{{json}}
```

If a parameter is passed it prints only the given variable:

```handlebars
{{json context}}
```

### Table of Contents

Use the `toc` helper to iterate the index of a directory:

```handlebars
{{#toc}}
<li><a href="{{href}}">{{title}}</a></li>
{{/toc}}
```

The computed data for each destination is available and `href` which points to the destination for the source file.

### HTML

The `html` helper is designed to be used in markdown documents; it renders an element with attributes and processes the inner block as markdown.

Often when writing markdown document we just want to add an element with a `class` attribute but as soon as we do everything must be HTML which is quite inconvenient. This allows us to add wrapper elements and continue to write markdown inside.

Pass the tag name as a string and an optional object of attributes:

```handlebars
{{#html "section"}}
A section element containing some inline _markdown_.
{{/html}}

{{#html "div" {"class": "note"}}}
A div element with a `class` so that we can ***style this content easily***.
{{/html}}
```

Tag names and attribute values ***are not escaped***, it is assumed you know what you are doing.

Be aware that the inner block of markdown is parsed outside of the document scope and ***cannot use link references*** in the containing document.

## Clean URLs

When the `--clean-url` option is given treat destination files as clean URLs wherever possible (see [contact](/site/contact.html)). Does not apply to files generated via [mdbook][].

## Help

Get short help with `hypertext -h` and see more detail with `hypertext --help`.

## Notes

Much inspiration lifted from [mdbook][].

## License

See [LICENSE](/LICENSE).

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
