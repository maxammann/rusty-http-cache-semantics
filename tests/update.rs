use http::header::HeaderName;
use http::request::Parts as RequestParts;
use http::{header, HeaderMap, Request, Response};
use http_cache_semantics::CachePolicy;
use std::time::SystemTime;

fn request_parts(builder: http::request::Builder) -> http::request::Parts {
    builder.body(()).unwrap().into_parts().0
}

fn response_parts(builder: http::response::Builder) -> http::response::Parts {
    builder.body(()).unwrap().into_parts().0
}

fn simple_request_builder_for_update(
    additional_headers: Option<HeaderMap>,
) -> http::request::Builder {
    let mut builder = Request::builder()
        .header(header::HOST, "www.w3c.org")
        .header(header::CONNECTION, "close")
        .uri("/Protocols/rfc2616/rfc2616-sec14.html");

    let builder_headers = builder.headers_mut().unwrap();
    if additional_headers.is_some() {
        for (key, value) in additional_headers.unwrap() {
            builder_headers.insert(key.unwrap(), value);
        }
    }

    builder
}

fn cacheable_response_builder_for_update() -> http::response::Builder {
    Response::builder().header(header::CACHE_CONTROL, "max-age=111")
}

fn etagged_response_builder() -> http::response::Builder {
    cacheable_response_builder_for_update().header(header::ETAG, "\"123456789\"")
}

fn request_parts_from_headers(headers: HeaderMap) -> RequestParts {
    let mut builder = Request::builder();

    for (key, value) in headers {
        match key {
            Some(x) => {
                builder.headers_mut().unwrap().insert(x, value);
            }
            None => (),
        }
    }

    request_parts(builder)
}

fn not_modified_response_headers_for_update(
    first_request_builder: http::request::Builder,
    first_response_builder: http::response::Builder,
    second_request_builder: http::request::Builder,
    second_response_builder: http::response::Builder,
) -> Option<HeaderMap> {
    let now = SystemTime::now();
    let policy = CachePolicy::new(
        &request_parts(first_request_builder),
        &response_parts(first_response_builder),
        Default::default(),
    );

    let headers = policy
        .revalidation_request(&request_parts(second_request_builder))
        .headers;

    let rev = policy.revalidated_policy(
        &request_parts_from_headers(headers),
        &response_parts(second_response_builder),
        now,
    );

    if rev.modified {
        return None;
    }

    Some(rev.policy.cached_response(now).headers)
}

fn assert_updates(
    first_request_builder: http::request::Builder,
    first_response_builder: http::response::Builder,
    second_request_builder: http::request::Builder,
    second_response_builder: http::response::Builder,
) {
    let extended_second_response_builder = second_response_builder
        .header(HeaderName::from_static("foo"), "updated")
        .header(HeaderName::from_static("x-ignore-new"), "ignoreme");
    let etag_built = extended_second_response_builder
        .headers_ref()
        .unwrap()
        .get(header::ETAG)
        .unwrap()
        .clone();

    let headers = not_modified_response_headers_for_update(
        first_request_builder,
        first_response_builder
            .header(HeaderName::from_static("foo"), "original")
            .header(HeaderName::from_static("x-other"), "original"),
        second_request_builder,
        extended_second_response_builder,
    )
    .expect("not_modified_response_headers_for_update");

    assert_eq!(headers.get("foo").unwrap(), "updated");
    assert_eq!(headers.get("x-other").unwrap(), "original");
    assert!(headers.get("x-ignore-new").is_none());
    assert_eq!(headers.get(header::ETAG).unwrap(), etag_built);
}

#[test]
fn test_matching_etags_are_updated() {
    assert_updates(
        simple_request_builder_for_update(None),
        etagged_response_builder(),
        simple_request_builder_for_update(None),
        etagged_response_builder().status(http::StatusCode::NOT_MODIFIED),
    );
}
