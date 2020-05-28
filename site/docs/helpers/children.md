{{> components}}

Use the `children` helper to iterate the destinations for the current location:

```handlebars
{{{{raw}}}}{{#children}}
* [{{title}}]({{href}})
{{/children}}{{{{/raw}}}}
```

The computed data for each destination is included and `href` which points to the destination for each child entry.

The `self` boolean allows us to ignore the current location which is useful for building an index of the current directory:

```handlebars
{{{{raw}}}}{{#children}}
{{#unless self}}
* [{{title}}]({{href}})
{{/unless}}
{{/children}}{{{{/raw}}}}
```

[Back to helpers](..)
