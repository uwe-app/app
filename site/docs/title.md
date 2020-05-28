{{> components}}

A page title is inferred from the file name and assigned to the page template data unless a specific title has been defined; when a page is an index file the name of the parent directory is used instead.

To set a custom title for a page, define it in the [data]({{ link "/docs/data" }}):

```toml
["section/page"]
title = "Custom Page Title"
```

[Back to documentation](..)
