## Template Data

Create a file with the same name and `.toml` extension for document-specific configuration.

Data is loaded by searching for `layout.toml` in the current directory and parents, if a file is found it is used before merging with any document-specific configuration file.

### Reserved Keywords

These fields are **reserved** keywords for template data:

* `context` Helper context information.
* `template` Rendered document content (layouts only).

### Configure

Fields available to configure processing:

* `title` Document title.
* `standalone` Document is standalone.

When the `standalone` field is `true` the document will skip layout processing. Be aware if you set this field in the top-level `layout.toml` then no layouts would be used due to configuration inheritance.

### Custom

You can add custom configuration data and access it in the document template.

