+++
title = "Deck"

[[links]]
rel = "stylesheet"
href = "/assets/styles/deck.css"
+++

<div class="reveal">
  <div class="slides">
    {{#menu "slides"~}}
<section>
<span class="slide-title">{{this.title}}</span>
{{render this.href}}
</section>
    {{~/menu}}
  </div>
</div>

<noscript>
<p>
This presentation is best viewed with Javascript enabled; view [[all|all the presentation]] slides.
</p>
</noscript>
