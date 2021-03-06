+++
title = "Presentation"

[[links]]
rel = "stylesheet"
href = "/assets/styles/all.css"

[[links]]
rel = "stylesheet"
href = "/assets/styles/print.css"
media = "print"
+++

{{#menu "slides"}}
<h2>{{this.title}}</h2>
<section>{{render this.href}}</section>
{{/menu}}

<style>
main {
  align-items: flex-start !important;
}
</style>
