## Home

{{> intro.md}}

<pre>
{{json}}
</pre>

This is a *document* with some _markdown_ including an [example][] link reference.

You can view an example book at the [guide](/guide/).

<ul>
{{#toc}}
<li><a href="{{href}}">{{title}}</a></li>
{{/toc}}
</ul>

{{#html "div" {"class": "note \"another\""}}}
This is some text with inline _markdown_.

And some more paragraphs...that you can read...
{{/html}}

[example]: https://example.org 
