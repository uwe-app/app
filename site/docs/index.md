{{> components}}

{{#children}}
{{#unless self}}
* [{{title}}]({{href}})
    <p>{{description}}</p>
{{/unless}}
{{/children}}
