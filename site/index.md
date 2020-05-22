Super fast, opinionated, site generator combining [pulldown-cmark][], [handlebars][] and [mdbook][].

These are the processing rules at a glance; more information in the [docs](/docs/).

---

* Treat `.html` and `.md` files as *documents* to be parsed as templates.
* Load template [data](/docs/data/) from the `site/data.toml` file.
* Use the nearest [layout](/docs/layout/) for pages unless [standalone](/docs/standalone/).
* Load [partials](/docs/partials/) from `site/template`.
* Infer document title from the file name (or parent directory in the case of index files).
* If a directory contains `book.toml` compile as a [book](/docs/book/).
* [Ignore](/docs/ignore/) `site/template` directory, hidden files and patterns in `.gitignore`.
* Copy all other files.

---

### Usage

Process all files in `site` and write the result to `build/release`.

```
ht --release
```

```
{{ include output.txt }}
```

For a debug version run `ht`; your website is now in `build/debug`.


Get short help with `ht -h` and see more detail with `ht --help`; information on options is in the [command line][] docs.

[overview]: /overview/
[download]: /download/
[command line]: /docs/command-line/
[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
