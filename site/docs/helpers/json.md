## JSON

A useful helper to pretty print the template data as JSON.

```handlebars
{{{{raw}}}}{{json}}{{{{/raw}}}}
```

```json
{{json}}
```

If a parameter is passed it prints only the given variable:

```handlebars
{{{{raw}}}}{{json context}}{{{{/raw}}}}
```

```json
{{json context}}
```

{{#parent}}
[Back to helpers]({{href}})
{{/parent}}
