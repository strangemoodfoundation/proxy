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
    to: String,
}

async fn build_proxy(port: u16, to: String) -> Result<SimpleProxy, Box<dyn std::error::Error>> {
    let mut proxy = SimpleProxy::new(port, Environment::Production);
    let auth = router::Router::new(vec![
        Route {
            from: RouteRegex {
                host: Regex::new("^(.*)$").unwrap(),
                path: Regex::new("^/api/v0/add(.*)$").unwrap(),
            },
            to: RouteString {
                host: to.clone(),
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
                host: to.clone(),
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

    Ok(proxy)
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Cli::from_args();

    let proxy = build_proxy(args.port, args.to).await.unwrap();

    println!("Starting server at {}:{}", "localhost", args.port);
    spawn(async move { proxy.run().await });

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }
}
