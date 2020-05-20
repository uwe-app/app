## Helpers

Helpers are here to make life easier.

### JSON

A useful helper to pretty print the JSON template data.

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

### Table of Contents

Use the `toc` helper to iterate the index of a directory:

```handlebars
{{{{raw}}}}{{#toc}}
<li><a href="{{href}}">{{title}}</a></li>
{{/toc}}{{{{/raw}}}}
```

The computed data for each destination is available and `href` which points to the destination for the source file.

### HTML

The `html` helper is designed to be used in markdown documents; it renders an element with attributes and processes the inner block as markdown.

Often when writing markdown document we just want to add an element with a `class` attribute but as soon as we do everything must be HTML which is quite inconvenient. This allows us to add wrapper elements and continue to write markdown inside.

Pass the tag name as a string and an optional object of attributes:

```handlebars
{{{{raw}}}}{{#html "section"}}
A section element containing some inline _markdown_.
{{/html}}

{{#html "div" {"class": "note"}}}
A div element with a `class` so that we can ***style this content easily***.
{{/html}}{{{{/raw}}}}
```

Tag names and attribute values ***are not escaped***, it is assumed you know what you are doing.

Be aware that the inner block of markdown is parsed outside of the document scope and ***cannot use link references*** in the containing document.

