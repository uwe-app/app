{{> components}}

Drafts allow you to work on documents and books but exclude them from release builds.

For pages set `draft` for a page in the `data.toml` file:

```toml
["section/page"]
draft = true
```

For books add it to a `[site]` table in the `book.toml` file:

```toml
[site]
draft = true

[book]
# book configuration goes here
```

[Back to documentation](..)
