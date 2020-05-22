## Tree

Use the `toc` helper to iterate the index of a directory:

```handlebars
{{{{raw}}}}{{#toc}}
<li><a href="{{href}}">{{title}}</a></li>
{{/toc}}{{{{/raw}}}}
```

The computed data for each destination is available and `href` which points to the destination for the source file.

