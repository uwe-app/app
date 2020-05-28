## Websocket Security

Recent news has shown that some malicious websites are abusing local websocket servers (you can read more [here](https://news.ycombinator.com/item?id=23246170) and [here](https://news.ycombinator.com/item?id=23256458)).

This article looks at how we have mitigated these attacks using several methods; we assume you are using the default host and port `ht --live`.

First we set [CORS][] on the websocket endpoint which means that if you are running a live reload server you must use the host name (default value is `localhost`).

Contrary to many comments online it appears that browsers do respect [CORS][] headers. In tests using `Chrome 62.0.3202.75` and `Firefox 76.0` attempts to access the server at the address `127.0.0.1:3000` will load the pages but the live reload websockets fail with a `403` error .

> CORS headers are not used when assigning an ephemeral port (`ht --live --port=0`)

More importantly we implement the [synchronizer token pattern](https://en.wikipedia.org/wiki/Cross-site_request_forgery#Synchronizer_token_pattern) using a random value for the websocket endpoint. This value is 16-bytes randomly generated and converted to hexadecimal, the code looks like:

```rust
pub fn generate_id(len: i32) -> String {
    let mut s: String = "".to_owned();
    for _ in 0..len {
        let x = rand::random::<u8>();
        s.push_str(&format!("{:x}", x));
    }
    s
}
```

The generated token is used as the path for the websocket server and is embedded into templates using the `livereload` parameter.

Once templates are rendered they end up connecting to this random endpoint which results in a connection string like:

```
ws://localhost:3000/f46011997b3c2f9ed4f72691840a287
```

The `u8` type has 256 possible values and we are using 16 or them when we generate the endpoint identifier which gives us `256^16` possible values or `3.402823669209385e+38`! This makes it incredibly difficult for any malicious actors to locate the endpoint for the live reload websocket server.

Unless the page was rendered by the `ht` web server there is very little chance of knowing the websocket server endpoint!

[Back to articles](..)

[CORS]: https://en.wikipedia.org/wiki/Cross-origin_resource_sharing
