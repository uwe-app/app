# Hyper Text

A program for combining HTML and [Commonmark][] seamlessly using [Handlebars][] for templating.

```
hypertext source target
```

Read all files in source and process them according to the following rules:

* `.htm`, `.html`, `.hbs`: Parse as HTML template.
* `.md`, `.markdown`: Parse as Markdown template.
* Treat `hypertext.hbs` files as master templates.
* Skip any files matched by ignore patterns (`--ignore`).
* Copy all other files to `target`.

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

[Commonmark]: https://commonmark.org/
[Handlebars]: https://handlebarsjs.com/
