## Usage

Once you have hypertext [installed](/install/) you can get help with `ht --help` and more about each command using `ht help <cmd>`.

As a convenience the most common command `build` is run when no command is given.

### Init

To create a new website with some skeleton files use the `init` command:

```
ht init website
```

Which will create the folder `website` and all the basic files using the default template. To see available template names run `ht init --list`.

Once you find one you prefer you can use it with the template option:

```
ht init --template=tacit website
```

### Build

Now you can enter the new website and create a build:

```
cd website
ht
```

Which will process all files in `site` and write the result to `build/debug`. For a release version the files are written to `build/release`:

```
ht --release
```

### Live Reload

To live reload files in the browser as you work pass the `--live` option:

```
ht --live
```

### Filters

Hypertext is designed to be used on very large sites and offers several options for controlling what to compile.

You can use the `--directory` option to only build a sub-directory; the path must be relative to the input directory.

To only build files in the `site/docs` directory:

```
ht --directory=docs
```

Combine this with the `--max-depth` option to control the recursion levels; for example to only build the top-level documentation:

```
ht --directory=docs --max-depth=1
```

Or only the top-level pages for the site just:

```
ht --max-depth=1
```

Careful, if you pass `--max-depth=0` nothing is compiled!

### Help

---

```
{{include help.txt}}
```

---

