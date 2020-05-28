{{> components}}

You may wish to ignore certain files from the `site` directory; by default the entire `template` directory is excluded for obvious reasons.

In addition hidden files will be ignored and any `.gitignore` files are respected.

If you need to include files that have been excluded by `.gitignore` add an `.ignore` file which will take precedence.

For example this website has a files directory with binary executables for download. We want to ignore these files from git but include them in the website. First we ignore all the files from git (`.gitignore`):

```
ht-*
```

Then override the pattern in `.ignore`:

```
!ht-*
```

Job done!

[Back to documentation](..)
