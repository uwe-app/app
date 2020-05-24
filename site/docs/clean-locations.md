## Clean Locations

It is prettier to use clean locations wherever possible so the default behaviour is to generate directories with an `index.html` file.

If you would prefer your pages to use the `.html` file extension you can specify the `--html-extension` option. Be aware that these two modes of operation are not interchangeable as they affect how you link your pages together.

We think clean locations are better which is why they are enabled by default; however there is a caveat to be aware of.

Consider the source file `about.md`:

```
site
└── about.md
```

When a build is created the resulting file is here:

```
build/debug
└── about
    └── index.html
```

Which means we can use the `/about/` location when writing links in our website. But if we use a folder and an index file too:

```
site
├── about
│   └── index.md
└── about.md
```

There is a problem because `about.md` cannot be mapped to the index file which is already declared as `about/index.md`. In this instance the specific index file takes precedence and the file that cannot be treated as a clean location is written to `about.html`:

```
build/debug
├── about
│   └── index.html
└── about.html
```

If you design your site structure to avoid these naming conflicts then clean locations should work just fine!
