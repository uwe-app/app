## Bundles: Launch

In the [first part]({{ link "../bundles-introduction/" }}) of this series on bundles we looked at how to generate bundles. Now let's examine what happens when you launch the executable and how files are bundled.

When a bundle executable is launched it binds a web server to an ephemeral port, serves the bundled files from that address and opens a browser window with the website.

Launching a bundle using a fixed port would be too error-prone so an ephemeral port is used by default, however sometimes you might wish to serve the website on a fixed port in which case you can pass a specific bind address:

```
hypertext-linux :8000
```

Read on if you want more detail on how bundles are created.

During bundle creation all the files in the source directory are read and converted to byte arrays which are dynamically written to `assets.go`. The [Go][] compiler then includes those files directly in the resulting executable and because the generated code implements the [http.Dir][] and [http.File][] interfaces the built in web server can read and send the website files!

There is not much code involved (~200 lines), if you are interested run with `--keep` and take a look at the intermediary code in `build/bundle`:

```
ht bundle build/release --force --keep
```

Apart from having a web server as part of the standard library the other reason [Go][] is the right choice for this task is because of it's excellent cross-compilation abilities!

{{#parent}}
[Back to articles]({{href}})
{{/parent}}

[Go]: https://golang.org/
[http.Dir]: https://golang.org/pkg/net/http/#Dir.Open
[http.File]: https://golang.org/pkg/net/http/#File
