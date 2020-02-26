use http::header::HOST;
use http::Uri;
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};
use hyper_tls::HttpsConnector;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or("8080".into());
    let upstream = env::var("UPSTREAM").unwrap();
    let addr = SocketAddr::from(([0, 0, 0, 0], port.parse().unwrap()));
    let options_header = OptionsHeader {
        allow_origin: env::var("OPTIONS_ALLOW_ORIGIN").unwrap_or_default(),
        allow_methods: env::var("OPTIONS_ALLOW_METHODS").unwrap_or_default(),
        allow_headers: env::var("OPTIONS_ALLOW_HEADERS").unwrap_or_default(),
        allow_credentials: env::var("OPTIONS_ALLOW_CREDENTIALS").unwrap_or_default(),
        expose_headers: env::var("OPTIONS_EXPOSE_HEADERS").unwrap_or_default(),
        max_age: env::var("OPTIONS_MAX_AGE").unwrap_or_default(),
    };
    let http_client = Client::builder().build::<_, hyper::Body>(HttpConnector::new());
    let https_client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

    let make_svc = make_service_fn(move |_| {
        let http_client = http_client.clone();
        let https_client = https_client.clone();
        let upstream = upstream.clone();
        let options_header = options_header.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                proxy(
                    upstream.clone(),
                    options_header.clone(),
                    (http_client.clone(), https_client.clone()),
                    req,
                )
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

#[derive(Clone, Debug)]
struct OptionsHeader {
    allow_origin: String,
    allow_methods: String,
    allow_headers: String,
    allow_credentials: String,
    expose_headers: String,
    max_age: String,
}

async fn proxy(
    upstream: String,
    options_header: OptionsHeader,
    clients: (Client<HttpConnector>, Client<HttpsConnector<HttpConnector>>),
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    println!("{} {}", req.method(), req.uri().path());

    if req.method() == "OPTIONS" {
        let mut resp_builder = Response::builder().status(204);
        if !options_header.allow_origin.is_empty() {
            resp_builder =
                resp_builder.header("Access-Control-Allow-Origin", options_header.allow_origin);
        }
        if !options_header.allow_methods.is_empty() {
            resp_builder =
                resp_builder.header("Access-Control-Allow-Methods", options_header.allow_methods)
        }
        if !options_header.allow_headers.is_empty() {
            resp_builder =
                resp_builder.header("Access-Control-Allow-Headers", options_header.allow_headers);
        }
        if !options_header.allow_credentials.is_empty() {
            resp_builder = resp_builder.header(
                "Access-Control-Allow-Credentials",
                options_header.allow_credentials,
            );
        }
        if !options_header.max_age.is_empty() {
            resp_builder = resp_builder.header("Access-Control-Max-Age", options_header.max_age);
        }
        return Ok(resp_builder.body(Body::empty()).unwrap());
    }

    let (http_client, https_client) = clients;

    let target_uri = (upstream.clone() + req.uri().path_and_query().unwrap().as_str())
        .parse()
        .unwrap();
    let (parts, body) = req.into_parts();
    let upstream_uri: Uri = upstream.clone().parse().unwrap();
    let mut upstream_req = Request::from_parts(parts, body);
    *upstream_req.uri_mut() = target_uri;
    upstream_req
        .headers_mut()
        .insert(HOST, upstream_uri.host().unwrap().parse().unwrap());

    let result = if upstream.starts_with("https://") {
        https_client.request(upstream_req).await
    } else {
        http_client.request(upstream_req).await
    };
    match result {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            let mut resp = Response::from_parts(parts, body);
            if !options_header.allow_origin.is_empty() {
                resp.headers_mut().insert(
                    "Access-Control-Allow-Origin",
                    options_header.allow_origin.parse().unwrap(),
                );
            }
            if !options_header.allow_credentials.is_empty() {
                resp.headers_mut().insert(
                    "Access-Control-Allow-Credentials",
                    options_header.allow_credentials.parse().unwrap(),
                );
            }
            if !options_header.expose_headers.is_empty() {
                resp.headers_mut().insert(
                    "Access-Control-Expose-Headers",
                    options_header.expose_headers.parse().unwrap(),
                );
            }
            Ok(resp)
        }
        Err(e) => Err(e),
    }
}
