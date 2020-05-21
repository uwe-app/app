Super fast, opinionated, site generator combining [pulldown-cmark][], [handlebars][] and [mdbook][].

```
ht
```

Process all files in `site` and write the result to `build/debug`.

```
{{> output.md}}
```

For a production version run `ht --release`; your website is now in `build/release`.

Get short help with `ht -h` and see more detail with `ht --help`, more information on program options in the [command line][] docs.

[command line]: /docs/command-line/
[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
