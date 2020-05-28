## Partials

Partial templates (`.hbs`)  in the `site/template` directory are automatically registered using a relative path without the file extension. So the file `site/template/header.hbs` can be included with:

```handlebars
{{{{raw}}}}{{> header}}{{{{/raw}}}}
```

## Snippets

Markdown documents are parsed via handlebars before being rendered to HTML which allows document snippets using the normal handlebars syntax. The referenced templates will also be parsed as markdown so we recommend using the `.md.hbs` file extension to distinguish them; the `.hbs` extension is always removed so you can reference snippets like so:

```handlebars
{{{{raw}}}}{{> snippet.md}}{{{{/raw}}}}
```

[Back to documentation](..)
