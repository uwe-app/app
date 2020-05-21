## Release Tags

The default release tag is `debug` and the `--release` option will generate a release build into `build/release`.

A release implies that all available optimizations are used; currently this is just minifying the HTML output but the plan is to integrate more optimizations later.

You may want to output to a different build directory for certain versions in which case you can use the tag option:

```
ht --tag=v3.1.0-alpha1
```

Which will generate a debug build in `build/v3.1.0-alpha1`; if you want a release version in the target directory you can combine `--tag` with `--release`.

```
ht --tag=v3.1.0-alpha1 --release
```
