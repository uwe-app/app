# Hyper Text

Super fast, opinionated, site generator combining [pulldown-cmark][], [handlebars][] and [mdbook][].

```
ht
```

> Process all files in `site` and write the result to `build`.

* Treat `.html` and `.md` files as *documents* to be parsed as templates.
* Use the nearest layout (`layout.hbs`) wherever possible.
* Infer document title from the file name (or parent directory in the case of index files).
* Load template data from `.toml` files.
* If a directory contains `book.toml` use [mdbook][].
* If the directory `site/template/theme` exists use it as the theme for [mdbook][].
* Exclude hidden files and patterns in `.gitignore`.
* Copy all other files.

Get short help with `ht -h` and see more detail with `ht --help`.

## TODO

* Support `ho(1)` for optimization pass to compress HTML etc.

## Notes

Much inspiration lifted from [mdbook][].

## License

See [LICENSE](/LICENSE).

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
