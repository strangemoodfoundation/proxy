use std::borrow::Cow;

use http::uri::{Parts, Uri};
use http::StatusCode;
use hyper::header::HeaderValue;

use hyper::{Body, Request};
use regex::Regex;
use simple_proxy::proxy::{
    error::MiddlewareError,
    middleware::{Middleware, MiddlewareResult},
    service::{ServiceContext, State},
};

#[derive(Clone, Default)]
pub struct Router {
    routes: RouterRules,
}

#[derive(Debug, Clone)]
pub struct RouteRegex {
    pub host: Regex,
    pub path: Regex,
}

#[derive(Debug, Clone)]
pub struct RouteString {
    pub host: String,
    pub path: String,
}

#[derive(Clone)]
pub struct Route {
    pub from: RouteRegex,
    pub to: RouteString,
    pub rule: fn(&Request<Body>) -> bool,
}

#[derive(Clone)]
pub struct RouterRulesWrapper {
    pub rules: RouterRules,
}

pub type RouterRules = Vec<Route>;

fn get_host_and_path_and_query(
    req: &mut Request<Body>,
) -> Result<(String, String, String), MiddlewareError> {
    let uri = req.uri();
    let path = uri
        .path_and_query()
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from(""));

    let query = uri
        .query()
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from(""));

    match uri.host() {
        Some(host) => Ok((String::from(host), path, query)),
        None => Ok((
            String::from(
                req.headers()
                    .get("host")
                    .unwrap_or(&HeaderValue::from_static(""))
                    .to_str()?,
            ),
            path,
            query,
        )),
    }
}

// redirects the request to the URI of the request from `old_host` to `new_host`,
// with the `/path`.
fn forward(
    req: &mut Request<Body>,
    old_host: &str,
    host: &str,
    path: &str,
) -> Result<(), MiddlewareError> {
    {
        let headers = req.headers_mut();

        headers.insert("X-Forwarded-Host", HeaderValue::from_str(old_host)?);
        headers.insert("host", HeaderValue::from_str(host)?);
    }
    let mut parts = Parts::default();
    parts.scheme = Some("http".parse()?);
    parts.authority = Some(host.parse()?);
    parts.path_and_query = Some(path.parse()?);

    *req.uri_mut() = Uri::from_parts(parts)?;

    Ok(())
}

impl Middleware for Router {
    fn before_request(
        &mut self,
        req: &mut Request<Body>,
        _context: &ServiceContext,
        _state: &State,
    ) -> Result<MiddlewareResult, MiddlewareError> {
        let (host, path, query) = get_host_and_path_and_query(req)?;
        println!("{}, {}, {}", host, path, query);

        let routes = &self.routes;

        for route in routes {
            let (re_host, re_path) = (&route.from.host, &route.from.path);
            let to = &route.to;

            if re_host.is_match(&host) {
                println!("is match {}", &host);
                // Check if it matches the rule
                if !(route.rule)(req) {
                    // TODO: maybe rule should return a middleware error?
                    return Err(MiddlewareError::new(
                        String::from("Unauthorized"),
                        Some(String::from("Unauthorized")),
                        StatusCode::UNAUTHORIZED,
                    ));
                }

                let new_host = re_host.replace(&host, to.host.as_str());

                let new_path = if re_path.is_match(&path) {
                    re_path.replace(&path, to.path.as_str())
                } else {
                    continue;
                };

                let combined = format!("{}?{}", new_path, query);
                let new_path = if query.is_empty() {
                    new_path
                } else {
                    Cow::from(combined.as_str())
                };

                println!("Proxying to {} {} ", &new_host, &new_path);
                forward(req, &host, &new_host, &new_path)?;

                println!("Req is now {:?}", req.uri());

                return Ok(MiddlewareResult::Next);
            } else {
                println!("is not match {}", &host);
            }
        }

        Err(MiddlewareError::new(
            String::from("No route matched"),
            Some(format!(
                "No route matched for {}, {}, {}",
                host, path, query
            )),
            StatusCode::NOT_FOUND,
        ))
    }

    fn name() -> String {
        "Auth".to_string()
    }
}

impl Router {
    pub fn new(routes: RouterRules) -> Self {
        Router { routes }
    }
}
