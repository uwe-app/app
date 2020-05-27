<p class="center">Beautiful web pages, incredibly fast</p>

Hypertext is a static site generator with [mdbook][] support designed for people that want to focus on the content; ideal for websites, technical documentation or a quick blog.

Use Markdown, HTML and CSS to build your site; add a sprinkle of Javascript if you need it.

These are the processing rules at a glance; more information in the [docs](/docs/).

---

* Treat `.html` and `.md` files as [pages](/docs/pages/) to be parsed as templates.
* Infer document [title](/docs/title/) from the file name.
* Load template [data](/docs/data/) from the `site/data.toml` file.
* Use a [layout](/docs/layout/) for pages unless [standalone](/docs/standalone/).
* Load [partials](/docs/partials/) from `site/template`.
* If a directory contains `book.toml` compile as a [book](/docs/book/).
* [Ignore](/docs/ignore/) `site/template` directory, hidden files and patterns in `.gitignore`.
* Copy all other files.

---

Interested? Head over to [install](/install/) then see [usage](/usage/) to get started.

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
