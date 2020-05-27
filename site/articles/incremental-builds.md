## Incremental Builds

If source files have not changed there is no need for us to render or copy them; a manifest is created for each [release tag](/docs/release-tags/) which is written to the build directory as a JSON file.

The manifest contains a list of source files and modification times; when a build runs it will compare modification times to determine if a file appears to have changed.

When you run `ht` on a fresh project you will see output like this:

```
INFO  ht > debug
INFO  hypertext::build > site/index.md -> build/debug/index.html
INFO  hypertext::build > site/assets/style.css -> build/debug/assets/style.css
INFO  ht               > 4.052078ms
```

If you run it again the output shows `noop`:

```
INFO  ht > debug
INFO  hypertext::build > noop site/index.md
INFO  hypertext::build > noop site/assets/style.css
INFO  ht               > 4.422663ms
```

If you want to force a full build you can use the `--force` option.

Note that when the `--live` option is given pages *must be compiled* to ensure they connect to the correct websocket server endpoint.

{{#parent}}
[Back to articles]({{href}})
{{/parent}}
