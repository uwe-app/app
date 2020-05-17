# Hyper Text

Fast, opinionated static site generation combining HTML, [commonmark][] with [handlebars][] and [mdbook][] support.

See [site](/site) for example files.

```
hypertext
```

Process all files in `site` and write the result to `build` using these rules:

* Parse `.html` and `.hbs` as HTML templates.
* Parse `.md` files as Markdown templates.
* For each parsed template walk the parent directroy structure, if a `hypertext.hbs` file is found use it as a master template passing each processed template as a `content` property.
* If a directory contains a `book.toml` file build using [mdbook][].
* If the directory matches `site/theme` treat as a global theme for [mdbook][].
* Skip any files matched by ignore patterns (`--ignore`).
* Copy all other files.

For each matched file if a `hypertext.hbs` file exists in any parent directory use it as a master template and generate the file content by parsing using the `hypertext.hbs` template file assigning the template markup as `content`.

A simple `hypertext.hbs` file might look like:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <title>{{title}}</title>
    <meta name="viewport" content="width=device-width,initial-scale=1">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
  </head>
  <body>
    {{{content}}}
  </body>
</html>
```

Notice the ***triple braces*** around `content` so the parsed template markup is not escaped.

## Options

If there are some files with those file extensions that you with to omit from processing use the `--ignore` option. It specifies a regular expression pattern tested on the entire file path, if the pattern matches the file will not be included:

```
hypertext source target --ignore ".*\.txt$"
```

You may specify multiple `--ignore` patterns if you need to.

[commonmark]: https://commonmark.org/
[handlebars]: https://handlebarsjs.com/
[mdbook]: https://rust-lang.github.io/mdBook/
