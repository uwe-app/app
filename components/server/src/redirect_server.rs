use warp::Filter;

pub fn spawn() {
    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move { run().await });
    });
}

pub async fn run() {
    let redirect = warp::get()
        .map(|| {
            warp::redirect(warp::http::Uri::from_static("/v2"))
        });

    warp::serve(redirect)
        .run(([127, 0, 0, 1], 8888))
        .await;
}
