use clap::Parser;
use reqwest::{blocking::Client, header};
use serde;
use std::io::Read;
use std::process::exit;
use std::time::Duration;

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(version, about = "", long_about = None)]
struct Config {
    /// name of your service
    name: String,
    /// optional commit to deploy (otherwise head of the default branch)
    commit: Option<String>,
    /// Wait for the deploy to finish or fail
    #[arg(short, long)]
    wait: bool,

    #[arg(short, long, env("RENDER_API_KEY"))]
    api_key: String,
}

#[derive(PartialEq, serde::Deserialize, Debug, Clone)]
struct Service {
    id: String,
    name: String,
    branch: String,
    dashboard_url: String,
    auto_deploy: bool,
    repo: String,
}

fn list_service(client: &Client, config: &Config) -> Option<Service> {
    let response = client
        .get("https://api.render.com/v1/services")
        .query(&[("name", config.name.clone()), ("limit", "1".to_string())])
        .send()
        .expect("Could not build request");
    if !response.status().is_success() {
        println!("Request Error: {:?} {:?}", response.status(), response.text().unwrap_or("Unknown Error".into()));
        exit(1);
    }
    let services: Vec<Service> = match response.json() {
        Ok(services) => services,
        Err(e) => {
            println!("Unable to parse json {}", e);
            exit(1);
        }
    };
    services.into_iter().next()
}

fn http_client(config: &Config) -> Client {
    let mut headers = header::HeaderMap::new();
    let bearer = format!("Bearer {}", config.api_key);
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&bearer).expect("valid api key"),
    );
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );

    Client::builder()
        .user_agent("render-deploy: https://github.com/reconbot/render-deploy")
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .gzip(true)
        .build()
        .expect("http client could be built")
}

fn main() {
    let config = Config::parse();
    let client = http_client(&config);
    // get the service
    let service = match list_service(&client, &config) {
        None => {
            println!("Cannot find a service named {}", config.name);
            exit(1);
        }
        Some(service) => service,
    };
    if service.auto_deploy {
        println!("Warning: AutoDeploy is true for {}", service.name)
    }

    println!("{:?}", service);

    // trigger deploy
    // if error error

    if config.wait {
        // poll for a good status or a bad status
        // if error error
    }
    // fin!

    println!(
        "deploying {:?} with commit {:?} and wait:{:?}",
        config.name, config.commit, config.wait
    );
}
