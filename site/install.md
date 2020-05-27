## Install

Binary executables are available to download for [Linux](/files/ht-gnu-linux-x86_64/ht) and [MacOS](/files/ht-darwin-x86_64/ht).

Ensure the downloaded file is executable with `chmod +x ht` and then put it in your `PATH` so you can run it from anywhere.

Next you can get started with this one-liner:

```
ht init project && (cd project && ht --live)
```

Which will perform the following tasks:

* Create a new website in the `project` folder
* Compile the files in `project/site` to `project/build/debug`
* Launch the site in a browser
* Watch the `project/site` directory for changes
    
You can get going right away; edit the files in `project/site` and check your changes in the browser. To learn more check out [usage](/usage/) and the [docs](/docs/).

