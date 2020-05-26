## Bundles: Launch

In the [first part](../bundles-introduction/) of this series on bundles we looked at how to generate bundles. Now let's examine how bundles are created and what happens when you launch the executable.

When a bundle executable is launched it binds a web server to an ephemeral port, serves the bundled files from that address and opens a browser window pointing to the bundled web server.

When launching a bundle using a fixed port would be too error-prone so an ephemeral port is used by default, however sometimes you might wish to serve the website on a fixed port in which case you can pass a specific bind address:

```
hypertext-linux :8000
```

{{#parent}}
[Back to articles]({{href}})
{{/parent}}
