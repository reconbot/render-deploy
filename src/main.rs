use clap::Parser;
use reqwest::{blocking::Client, header};
use serde;
use serde::Deserialize;
use serde_json;
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

fn deserialize_yes_no<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;

    match s {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(serde::de::Error::unknown_variant(s, &["yes", "no"])),
    }
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
struct Service {
    id: String,
    name: String,
    branch: String,
    #[serde(rename = "dashboardUrl")]
    dashboard_url: String,
    #[serde(rename = "autoDeploy", deserialize_with = "deserialize_yes_no")]
    auto_deploy: bool,
    repo: String,
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
struct ListServiceResponse {
    cursor: String,
    service: Service,
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

fn list_service(client: &Client, config: &Config) -> Option<Service> {
    let response = client
        .get("https://api.render.com/v1/services")
        .query(&[("name", config.name.clone()), ("limit", "1".to_string())])
        .send()
        .expect("Could not build request");
    if !response.status().is_success() {
        println!(
            "Request Error: {:?} {:?}",
            response.status(),
            response.text().unwrap_or("Unknown Error".into())
        );
        exit(1);
    }
    let body = response.text().expect("unable to read response body");

    let services: Vec<ListServiceResponse> = match serde_json::from_str(&body) {
        Ok(services) => services,
        Err(e) => {
            println!("Unable to parse json {:?}", e);
            println!("{}", body);
            exit(1);
        }
    };
    services.into_iter().next().map(|resp| resp.service)
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
struct CommitInfo {
    id: String,
    message: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum DeployStatus {
    Created,
    BuildInProgress,
    UpdateInProgress,
    Live,
    Deactivated,
    BuildFailed,
    UpdateFailed,
    Canceled,
    PreDeployInProgress,
    PreDeployFailed,
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
struct Deploy {
    id: String,
    commit: CommitInfo,
    status: DeployStatus,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "finishedAt")]
    finished_at: Option<String>,
}

fn trigger_deploy(client: &Client, service: &Service, config: &Config) -> Result<Deploy, String> {
    // todo json post commitId if present
    let response = client
        .post(format!(
            "https://api.render.com/v1/services/{}/deploys",
            service.id
        ))
        .send()
        .expect("Could not build request trigger_deploy");
    if !response.status().is_success() {
        println!(
            "Request Error: {:?} {:?}",
            response.status(),
            response.text().unwrap_or("Unknown Error".into())
        );
        exit(1);
    }
    let body = response.text().expect("unable to read response body");

    let deploy: Deploy = match serde_json::from_str(&body) {
        Ok(deploy) => deploy,
        Err(e) => {
            return Result::Err(format!("Unable to parse json {:?} {}", e, body));
        }
    };
    Result::Ok(deploy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_deploy() {
        let sample = r#"
            {
                "id": "dep-cs67ufi3esus73b74a70",
                "commit": {
                    "id": "b2be9cf9e3188d00f58ef18a5904528993faeaa2",
                    "message": "render uses sigterm and disable aws healthcheck",
                    "createdAt": "2024-10-11T20:02:45Z"
                },
                "status": "build_in_progress",
                "trigger": "api",
                "createdAt": "2024-10-14T02:17:35.868638Z",
                "updatedAt": "2024-10-14T02:17:35.868638Z",
                "finishedAt": null
            }
        "#;
        let deploy: Deploy = serde_json::from_str(sample).unwrap();
        assert_eq!(deploy.id, "dep-cs67ufi3esus73b74a70");
        assert_eq!(deploy.commit.id, "b2be9cf9e3188d00f58ef18a5904528993faeaa2");
        assert_eq!(deploy.status, DeployStatus::BuildInProgress);
        assert_eq!(deploy.finished_at, None);
    }
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
    let deploy = trigger_deploy(&client, &service, &config).unwrap();
    println!("{:?}", deploy);

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
