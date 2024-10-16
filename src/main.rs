use clap::Parser;
use reqwest::{blocking::Client, header};
use serde;
use serde::Deserialize;
use serde_json;
use std::fmt;
use std::process::exit;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(version, about = " I needed a cli for render.com and I wanted to play with rust so it's a rust cli for triggering deploys on render.com", long_about = None)]
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

    /// wait for deploy timeout in seconds, doesn't cancel the deploy just exits
    #[arg(short, long, default_value="600", value_parser = parse_duration)]
    timeout: Duration,
}

fn parse_duration(arg: &str) -> Result<Duration, std::num::ParseIntError> {
    let seconds = arg.parse::<u64>()?;
    Ok(Duration::from_secs(seconds))
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
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "createdAt")]
    created_at: String,
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

impl fmt::Display for DeployStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            DeployStatus::Created => "Created",
            DeployStatus::BuildInProgress => "Build In Progress",
            DeployStatus::UpdateInProgress => "Update In Progress",
            DeployStatus::Live => "Live",
            DeployStatus::Deactivated => "Deactivated",
            DeployStatus::BuildFailed => "Build Failed",
            DeployStatus::UpdateFailed => "Update Failed",
            DeployStatus::Canceled => "Canceled",
            DeployStatus::PreDeployInProgress => "Pre-Deploy In Progress",
            DeployStatus::PreDeployFailed => "Pre-Deploy Failed",
        };
        write!(f, "{}", status_str)
    }
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

fn deploy_url(service: &Service, deploy: &Deploy) -> String {
    format!(
        "https://dashboard.render.com/web/{service_id}/deploys/{deploy_id}",
        deploy_id = deploy.id,
        service_id = service.id
    )
}

#[derive(PartialEq, Deserialize, Debug, Clone)]
struct ListDeploysResponse {
    cursor: String,
    deploy: Deploy,
}

fn latest_deploy(client: &Client, service: &Service) -> Option<Deploy> {
    let response = client
        .get(format!(
            "https://api.render.com/v1/services/{}/deploys",
            service.id
        ))
        .query(&[("limit", "1".to_string())])
        .send()
        .expect("Could not build request latest_deploy");
    if !response.status().is_success() {
        println!(
            "Request Error: {:?} {:?}",
            response.status(),
            response.text().unwrap_or("Unknown Error".into())
        );
        exit(1);
    }
    let body = response.text().expect("unable to read response body");

    let deploys: Vec<ListDeploysResponse> = match serde_json::from_str(&body) {
        Ok(services) => services,
        Err(e) => {
            println!("Unable to parse json {:?}", e);
            println!("{}", body);
            exit(1);
        }
    };
    deploys.into_iter().next().map(|resp| resp.deploy)
}

fn get_deploy(client: &Client, service: &Service, deploy_id: &String) -> Option<Deploy> {
    let response = client
        .get(format!(
            "https://api.render.com/v1/services/{service_id}/deploys/{deploy_id}",
            service_id = service.id,
            deploy_id = deploy_id
        ))
        .query(&[("limit", "1".to_string())])
        .send()
        .expect("Could not build request latest_deploy");
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
        Ok(services) => services,
        Err(e) => {
            println!("Unable to parse json {:?}", e);
            println!("{}", body);
            exit(1);
        }
    };
    Some(deploy)
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
    println!(
        "Found {name} {dashboard}",
        name = service.name,
        dashboard = service.dashboard_url
    );
    if service.auto_deploy {
        println!("Warning: AutoDeploy is true");
    }

    if config.commit.is_some() {
        println!(
            "Deploying {repo} #{commit}",
            repo = service.repo,
            commit = config.commit.clone().unwrap()
        );
    } else {
        println!(
            "Deploying {repo} #{branch}",
            repo = service.repo,
            branch = service.branch
        );
    }
    print!("\n");

    let previous_deploy = latest_deploy(&client, &service);
    if previous_deploy.is_some() {
        let deploy = previous_deploy.unwrap();
        println!(
            "Previous Deploy {commit} - {message}",
            commit = deploy.commit.id,
            message = deploy.commit.message
        );
        println!(
            "Status: {status} on {finished_at}",
            status = deploy.status,
            finished_at = deploy.finished_at.unwrap_or("".into())
        );
        print!("\n");
    }

    // trigger deploy
    let deploy = trigger_deploy(&client, &service, &config).unwrap();
    println!(
        "Created Deploy #{commit} - {message}",
        commit = deploy.commit.id,
        message = deploy.commit.message
    );
    println!("{}", deploy_url(&service, &deploy));
    println!("Status: {status}", status = deploy.status);

    // if error error

    if config.wait {
        let start = Instant::now();
        loop {
            if start.elapsed() > config.timeout {
                println!("Deploy timed out");
                exit(1);
            }
            sleep(Duration::from_secs(5));
            let deploy = get_deploy(&client, &service, &deploy.id).unwrap();
            println!("Status: {status}", status = deploy.status);
            match deploy.status {
                DeployStatus::Live => {
                    println!(
                        "Deploy is live on {} in {} seconds",
                        deploy.finished_at.unwrap_or("unknown".into()),
                        start.elapsed().as_secs()
                    );
                    break;
                }
                DeployStatus::BuildInProgress
                | DeployStatus::UpdateInProgress
                | DeployStatus::PreDeployInProgress
                | DeployStatus::Created => (),
                DeployStatus::BuildFailed
                | DeployStatus::UpdateFailed
                | DeployStatus::Canceled
                | DeployStatus::Deactivated
                | DeployStatus::PreDeployFailed => {
                    println!(
                        "Deploy has Stopped {}",
                        deploy.finished_at.unwrap_or("unknown".into())
                    );
                    break;
                }
            }
        }
    }
    exit(0);
}
