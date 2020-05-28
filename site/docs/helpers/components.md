{{> components}}

The `components` helper can be used to create links to parent folders like the one above ^^.

Each iteration is passed the data for the target and some extra fields:

* `first`: Whether this is the first part.
* `last`: Whether this is the last part.
* `href`: A relative `../` reference to the ancestor.

The `first` field is useful if you want to skip the home page.

Here is the code that we use on this website:

```handlebars
<nav>
{{{{raw}}}}{{#components}}
{{#if last}}
{{title}}
{{else}}
<a href="{{href}}">{{title}}</a>
{{/if}}
{{/components}}{{{{/raw}}}}
</nav>
```

[Back to helpers](..)
