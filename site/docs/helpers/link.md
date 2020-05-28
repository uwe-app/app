## Link

The link helper generates links relative to the current page. You pass it an absolute path and it will return a URL relative to the current page with the correct amount of `../` references. 

```handlebars
[Home](\{{link "/"}})
```

If the above link were located at `/docs/helpers/link.md` it would correspond to the output URL (assuming [clean locations]({{link "/docs/clean-locations/"}})) `/docs/helpers/link/index.html` a URL of `../../../` would be generated.

It also does the important job of throwing an error when a destination URL does not exist which helps to prevent stale links!

If you pass a relative path without a leading slash or use the `http:` or `https:` schemes it will be not be modified.

[Back to helpers](..)
