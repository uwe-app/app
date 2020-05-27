## Bundles: Introduction

Bundles allow us to package up a website into a standalone executable. If publishing a preview version of a website is not ideal bundles can be a good way to get feedback on work in progress by sending it directly.

> Running executables can be dangerous you should only execute code you trust and send bundles to people that trust you

By default bundles for each major platform are generated using the [Go][] toolchain (which must be installed).

To generate a bundle first create a release version then pass the result to the bundle command:

```
ht --release
ht bundle build/release
```

On completion the program output will show:

```
INFO  hypertext::bundle          > hypertext-linux (linux amd64)
INFO  hypertext::bundle          > hypertext-linux 29.4 MB in 837.443846ms
INFO  hypertext::bundle          > hypertext-darwin (darwin amd64)
INFO  hypertext::bundle          > hypertext-darwin 29.3 MB in 891.330727ms
INFO  hypertext::bundle          > hypertext-windows.exe (windows amd64)
INFO  hypertext::bundle          > hypertext-windows.exe 29.1 MB in 844.123688ms
```

The generated files are now in the `build/bundle` directory; if the directory already exists you must remove it. Use the `--force` flag to remove the bundle directory before creation:

```
ht bundle build/release --force
```

The intermediary `.go` files are deleted by default; if you want to test the bundle locally you may want to keep them:

```
ht bundle build/release --force --keep
(cd build/bundle && go run .)
```

You can use the `--linux`, `--mac` and `--windows` options to only create bundles for certain platforms:

```
ht bundle build/release --force --linux --mac
```

The name for the generated executables is inferred from the name of the current working directory. You may set the executable name too:

```
ht bundle build/release --force --name=website-v3.1.0-alpha1
```

> Note that the final name will always include the platform identifier and only the `amd64` architecture is currently supported

Next lets look at what happens when a [bundle is launched](../bundles-launch/).

{{#parent}}
[Back to articles]({{href}})
{{/parent}}

[Go]: https://golang.org/
