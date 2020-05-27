## FAQ

Some frequently asked questions and relevant answers!

### Where can I get the source code?

This is not an open-source project but has various dependencies that are Apache-2 or MIT. This project is a larger work and we will release some source code to comply with those licenses, see [contact](/contact/).

### Why not open-source?

Because there are plans to create a commercial product whilst offering the main hypertext command line tools for free and we need to protect our interests.

### Why not use one of the myriad other tools?

Hypertext is incredibly fast and predicated on the idea that not every website needs to be a huge bundle of Javascript; many people still appreciate simple, clean pages with a focus on the content. 

### How did you make it so fast?

The speed is thanks to [rust][] and the projects listed in the [credits](/credits/).

### Do you support Javascript frameworks?

No. You can of course use your favourite Javascript framework as a pre-processor and write the files to the correct location for your website.

### Do you support CSS preprocessors?

No. See above, you can run any pre-processor you like and output the files to the correct location for your website.

### Can I bring my own template language?

No. It's just [handlebars][] with some useful [helpers](/docs/helpers/).

### What flavor of Markdown is supported?

We use [commonmark][] with support for a few non-standard features:

* [Strike through](https://github.github.com/gfm/#strikethrough-extension-)
* [Tables](https://github.github.com/gfm/#tables-extension-)
* [Task lists](https://github.github.com/gfm/#task-list-items-extension-)

### Where should I report bugs or request features?

Get in touch using our [contact](/contact/) details.

### Can I submit a theme?

Yes, if you have created a good classless CSS style file that you would like us to include please [get in touch](/contact/).

[handlebars]: https://handlebarsjs.com/
[commonmark]: https://commonmark.org/
[rust]: https://www.rust-lang.org/
