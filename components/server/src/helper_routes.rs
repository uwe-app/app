use std::str::FromStr;

use futures::future;

use warp::host::Authority;
use warp::{Filter, Rejection};

//use webdav_handler::warp::dav_dir;

/// Support for the Host header that also identifies the ephemeral port zero
/// and switches the test to only the host name if the expected host has
/// a port value of zero.
///
/// This allows ephemeral ports to work as expected with virtual hosts.
pub(crate) fn host_ephemeral(
    expected: &str,
) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    let expected =
        Authority::from_str(expected).expect("invalid host/authority");
    warp::host::optional()
        .and_then(move |option: Option<Authority>| match option {
            Some(authority) => {
                if authority == expected {
                    return future::ok(());
                } else {
                    if let Some(port) = expected.port() {
                        if port == 0 && authority.host() == expected.host() {
                            return future::ok(());
                        }
                    }
                }
                future::err(warp::reject::not_found())
            }
            _ => future::err(warp::reject::not_found()),
        })
        .untuple_one()
}

/*
/// Filter that always rejects so we can balance the filter trees.
pub(crate) fn none() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::path::end()
        .and_then(|| {
            future::err(warp::reject::not_found())
        })
        .untuple_one()
}
*/
