<p class="center">Beautiful web pages, incredibly fast</p>

Hypertext is a static site generator with [mdbook][] support designed for people that want to focus on the content; ideal for websites, technical documentation or a blog.

Use Markdown, HTML and CSS to build your site; add a sprinkle of Javascript if you need it.

These are the processing rules at a glance; more information in the [docs]({{ link "/docs/" }}).

---

* Treat `.html` and `.md` files as [pages]({{ link "/docs/pages/" }}) to be parsed as templates.
* Infer document [title]({{ link "/docs/title/" }}) from the file name.
* Load template [data]({{ link "/docs/data/" }}) from the `site/data.toml` file.
* Use a [layout]({{ link "/docs/layout/" }}) for pages unless [standalone]({{ link "/docs/standalone/" }}).
* Load [partials]({{ link "/docs/partials/" }}) from `site/template`.
* If a directory contains `book.toml` compile as a [book]({{ link "/docs/book/" }}).
* [Ignore]({{ link "/docs/ignore/" }}) `site/template` directory, hidden files and patterns in `.gitignore`.
* Copy all other files.

---

Interested? Head over to [install]({{ link "/install/" }}) then see [usage]({{ link "/usage/" }}) to get started.

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
