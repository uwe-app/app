## Layout

Layouts provide the skeleton structure for your pages; the default layout file (`layout.hbs`) should be at the root of the `site` source folder.

If you want to use a different layout for a particular page you can set the `layout` data for the file:

```toml
["install/index"]
layout = "install/layout.hbs"
```

If a page has been marked [standalone](/docs/standalone/) no layout is applied.

Layouts are passed the `template` variable which contains the HTML content for the page to be rendered and should render the content unescaped. The typical block to render a layout might include some partials and writing out the `template` in a `main` element like this:

```handlebars
{{{{raw}}}}
{{> header}}
<main>{{{template}}}</main>
{{> footer}}
{{{{/raw}}}}
```

{{#parent}}
[Back to documentation]({{href}})
{{/parent}}
