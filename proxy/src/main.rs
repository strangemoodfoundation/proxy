use regex::Regex;
use simple_proxy::{Environment, SimpleProxy};
use structopt::StructOpt;
use tokio::{signal, spawn};

use crate::router::{Route, RouteRegex, RouteString};

mod cors;
mod router;

#[derive(StructOpt, Debug)]
struct Cli {
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Cli::from_args();

    let mut proxy = SimpleProxy::new(args.port, Environment::Development);
    let auth = router::Router::new(vec![
        Route {
            from: RouteRegex {
                host: Regex::new("^(.*)$").unwrap(),
                path: Regex::new("^/api/v0/add(.*)$").unwrap(),
            },
            to: RouteString {
                host: "localhost:5001".to_string(),
                path: "/api/v0/add".to_string(),
            },
            rule: (|_| true),
        },
        Route {
            from: RouteRegex {
                host: Regex::new("^(.*)$").unwrap(),
                path: Regex::new("^/$").unwrap(),
            },
            to: RouteString {
                host: "localhost:5001".to_string(),
                path: "/".to_string(),
            },
            rule: (|_| true),
        },
    ]);

    let cors = cors::Cors::new(
        "*",
        "GET, POST, PUT, DELETE, OPTIONS",
        "Content-Type, Authorization",
    );

    proxy.add_middleware(Box::new(cors));
    proxy.add_middleware(Box::new(auth));

    println!("Starting server");
    spawn(async move { proxy.run().await });

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }
}
