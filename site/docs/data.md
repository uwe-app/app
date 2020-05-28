## Template Data

Template data is loaded from `site/data.toml`.

Define global variables in `[site]` and page-specific tables by relative path; page data is inherited from the site global data:

```toml
[site]
description = "Super fast, opinionated, site generator"
keywords = "Fast, Static, Rust, Website, Generator"

["/"]
title = "Home"

["docs/"]
title = "Documentation"
```

You do not need to specify the file extension for page-specific data but you should be sure to quote the path to prevent a TOML error. Paths are resolved relative to the source directory and a warning is printed if an unknown table is declared.

To reference the index file in a directory use a trailing slash; so `docs/` becomes shorthand for `docs/index`.

These fields are **reserved** keywords:

* `context` Helper context information.
* `livereload` Websocket URL for live reload.
* `template` Rendered document content (layouts only).

These fields are configurable:

* `title` Document title.
* `standalone` Document is standalone (see [standalone][]).
* `draft` Document has draft status (see [drafts][]).

Other than these keywords you may define any fields you like and they will be made available to your templates.

{{#parent}}
[Back to documentation]({{href}})
{{/parent}}

[standalone]: /docs/standalone
[drafts]: /docs/drafts
