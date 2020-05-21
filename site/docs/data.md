## Template Data

Data is loaded by first searching for `layout.toml` in the current directory and parents, if a file is found it is used before merging with any document-specific configuration file. This allows us to set global data in `site/layout.toml` and override variables for certain directories if necessary.

Document specific data is defined in a file using the same name and the `.toml` extension; this data take precedence over global and directory `layout.toml` files.

### Strict Mode

Templates are parsed with strict mode enabled so it is an error if a variable does not exist.

If you have **not** defined a variable in the root `layout.toml` but refer to it in a `layout.hbs` you can easily generate an error. A good practice to avoid this is to declare all your variables with default values in the `site/layout.toml` file.

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

