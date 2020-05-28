## Ephemeral Port

The use of ephemeral ports is very helpful when working with local development servers.

Often we may wish to be working on multiple websites and having to either close down another web server or keep specifying ports; which becomes a chore fast!

If this is the case for you just select an ephemeral port by specifying zero for the `--port` option:

```
ht --live --port=0
```

Or if you are just serving files:

```
ht serve build/release --port=0
```

The operating system will allocate the web server a free port when it attempts to bind and our code will operate on the actual socket address that was used.

[Back to articles](..)
