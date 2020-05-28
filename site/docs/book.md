{{> components}}

Support for [mdbook][] is first-class; if a `book.toml` file is found the directory is treated as a book and it is handled differently.

The book is compiled using [mdbook][] and the contents of the build directory are copied into the correct location for the website.

If the directory `site/template/theme` exists then it is configured as the theme directory for all books.

Note that this has not been tested when `build.build_dir` has been specified, currently the expectation is that books are generated in the default `book` directory.

[Back to documentation](..)

[mdbook]: https://github.com/rust-lang/mdBook

