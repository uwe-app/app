## Usage

Once you have hypertext [installed](/install/) you can get short help with `ht -h` and see more detail with `ht --help`.

---

```
{{include help.txt}}
```

---

### Debug

For a debug version run `ht` which will process all files in `site` and write the result to `build/debug`.

### Release

For a release version the files are written to `build/release`:

```
ht --release
```

Which produces the output:

```
{{ include output.txt }}
```

