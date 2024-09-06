use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub servers: Vec<Server>,
    pub error_pages: ErrorPages,
    pub limits: Limits,
    pub route: Routes,
}

#[derive(Deserialize, Debug)]
pub struct Server {
    pub address: String,
    pub port: u16,
    pub root: String,
    pub default_file: String,
    pub hostname: String,
}

#[derive(Deserialize, Debug)]
pub struct ErrorPages {
    pub error_404: String,
    pub error_405: String,
    pub error_500: String,
    pub error_400: String,
}

#[derive(Deserialize, Debug)]
pub struct Limits {
    pub client_body_size: usize,
}

#[derive(Deserialize, Debug)]
pub struct Routes {
    pub default: Route,
    pub other: Option<Route>,
}

#[derive(Deserialize, Debug)]
pub struct Route {
    pub methods: Vec<String>,
    pub root_directory: String,
    pub default: String,
    pub cgi: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Route_other {
    pub methods: Vec<String>,
    pub root_directory: String,
    pub default: String,
    pub cgi: Option<String>,
}

impl ServerConfig {
    pub fn from_file(file_path: &str) -> Self {
        let config_content = fs::read_to_string(file_path).unwrap_or_else(|err| {
            eprintln!("Failed to read configuration file: {}", err);
            std::process::exit(1);
        });
        toml::from_str(&config_content).unwrap_or_else(|err| {
            eprintln!("Failed to parse configuration file: {}", err);
            std::process::exit(1);
        })
    }
}