## Tree

Use the `tree` helper to iterate the site structure. By default it will list the destinations for the current location:

```handlebars
{{{{raw}}}}{{#tree}}
<li><a href="{{href}}">{{title}}</a></li>
{{/tree}}{{{{/raw}}}}
```

The computed data for each destination is available and `href` which points to the destination for the source file.

{{#parent}}
[Back to helpers]({{href}})
{{/parent}}
