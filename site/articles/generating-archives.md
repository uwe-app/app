## Generating Archives

The `archive` command is useful when you just want to save a copy of the website or distribute the build files and know that the recipient can run a local web server.

To generate an archive is easy:

```
ht archive build/release
```

Generates the archive `build/release.zip`. If you prefer you can pass a specific destination for the archive:

```
ht archive build/release archive/releases/v3.1.0-alpha1
```

Parent directories will be created where necessary and it will always be given the `.zip` extension; to overwrite an existing bundle pass the `--force` option.

{{#parent}}
[Back to articles]({{href}})
{{/parent}}
