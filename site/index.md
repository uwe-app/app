Fast and elegant site generator combining [pulldown-cmark][], [handlebars][] and [mdbook][].

These are the processing rules at a glance; more information in the [docs](/docs/).

---

* Treat `.html` and `.md` files as [pages](/docs/pages/) to be parsed as templates.
* Infer document [title](/docs/title/) from the file name.
* Load template [data](/docs/data/) from the `site/data.toml` file.
* Use the nearest [layout](/docs/layout/) for pages unless [standalone](/docs/standalone/).
* Load [partials](/docs/partials/) from `site/template`.
* If a directory contains `book.toml` compile as a [book](/docs/book/).
* [Ignore](/docs/ignore/) `site/template` directory, hidden files and patterns in `.gitignore`.
* Copy all other files.

---

Interested? Head over to [install](/install/) then see [usage](/usage/) to get started.

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
