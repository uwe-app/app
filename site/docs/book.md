## Book

Support for [mdbook][] is first-class. If a `book.toml` file is found the directory is treated as a book and it is handled differently.

The book is compiled using [mdbook][] and the contents of the build directory `book` are copied into the correct location for the website.

Note that this has not been tested when `build.build_dir` has been specified, currently the expectation is that books are generated in the default `book` directory.

[mdbook]: https://github.com/rust-lang/mdBook

{{#parent}}
[Back to documentation]({{href}})
{{/parent}}
