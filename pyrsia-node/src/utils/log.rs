use log::debug;
use std::convert::Infallible;
use warp::http::HeaderMap;
use warp::Filter;

pub fn log_headers() -> impl Filter<Extract = (), Error = Infallible> + Copy {
    warp::header::headers_cloned()
        .map(|headers: HeaderMap| {
            for (k, v) in headers.iter() {
                // Error from `to_str` should be handled properly
                debug!(target: "pyrsia_registry", "{}: {}", k, v.to_str().expect("Failed to print header value"))
            }
        })
        .untuple_one()
}
