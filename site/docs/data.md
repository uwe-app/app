## Template Data

Template data is loaded from `data.toml`.

Define global variables first then page-specific data in tables:

```toml
description = "Super fast, opinionated, site generator"
keywords = "Fast, Static, Rust, Website, Generator"

["index"]
title = "Home"

["docs/index"]
title = "Documentation"
```

You do not need to specify the file extension for page-specific data but you should be sure to quote the path to prevent a TOML error. Paths are resolved relative to the source directory and a warning is printed if an unknown table is declared.

These fields are **reserved** keywords:

* `context` Helper context information.
* `template` Rendered document content (layouts only).

These fields are configurable:

* `title` Document title.
* `standalone` Document is standalone.
* `draft` Document has draft status.

Other than these keywords you may define any fields you like and they will be made available to your templates.

### Notes

When the `standalone` field is `true` the document will skip layout processing. Be aware if you set this field at the top-level in `data.toml` then no layouts are applied.

For pages with `draft` set to `true` the output document is created except for release builds.

{{#parent}}
[Back to documentation]({{href}})
{{/parent}}
