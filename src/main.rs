#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate directories;
extern crate env_logger;
extern crate my_internet_ip;
extern crate reqwest;
extern crate toml;

use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;

use directories::ProjectDirs;

#[derive(Deserialize, Clone)]
struct Status {
    text: String,
    emoji: String,
}

#[derive(Deserialize)]
struct Location {
    ip: IpAddr,
    text: String,
    emoji: String,
}

#[derive(Deserialize)]
struct Config {
    token: String,
    defaults: Option<Status>,
    locations: Vec<Location>,
}

/// Get configuration from standard configuration directory:
///
/// * Linux: /home/alice/.config/slack-status/config.toml
/// * Mac: /Users/Alice/Library/Preferences/com.nsd.slack-status/config.toml
/// * Windows: C:\Users\Alice\AppData\Roaming\nsd\slack-status\config\config.toml
fn get_config_dir() -> Option<std::path::PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "nsd", "slack-status") {
        Some(proj_dirs.config_dir().to_path_buf())
    } else {
        None
    }
}

fn get_config() -> Option<Config> {
    if let Some(config_dir) = get_config_dir() {
        info!("Looking for configuration file in: {:?}", config_dir);

        let config_file_path = config_dir.join("config.toml");

        match File::open(config_file_path) {
            Ok(mut f) => {
                let mut contents = String::new();
                f.read_to_string(&mut contents)
                    .expect("something went wrong reading the file");

                let config = toml::from_str(contents.as_str()).unwrap();

                Some(config)
            }
            Err(e) => {
                warn!("Cannot find configuration file: {}.", e);
                None
            }
        }
    } else {
        warn!("Cannot find configuration directory.");
        None
    }
}

fn create_default_config(config_dir: &std::path::PathBuf) -> std::io::Result<()> {
    info!("Create sample config.toml at: {:?}", config_dir);

    let sample = include_str!("config.toml.sample");

    std::fs::create_dir_all(config_dir)?;
    let mut f = File::create(config_dir.join("config.toml"))?;
    f.write_all(sample.as_bytes())?;

    f.sync_all()?;
    Ok(())
}

/// Get status from configuration locations and current public IP.
fn get_status_from_location(locations: &[Location], ip: &IpAddr) -> Option<Status> {
    for location in locations {
        if location.ip == *ip {
            info!("{} => {}", location.ip, location.text);
            return Some(Status {
                text: location.text.clone(),
                emoji: location.emoji.clone(),
            });
        }
    }

    None
}

/// Get status from configuration locations and IP, or defaults.
fn get_status_from(config: Config, ip: &IpAddr) -> Status {
    match get_status_from_location(&config.locations, ip) {
        Some(status) => status,
        None => config.defaults.unwrap_or(Status {
            text: "on the move".to_string(),
            emoji: ":mountain_railway:".to_string(),
        }),
    }
}

fn main() {
    env_logger::init();

    debug!("Reading configuration...");
    let config = match get_config() {
        Some(config) => config,
        None => {
            println!("Configuration file not found!");
            let config_dir = get_config_dir().unwrap();
            create_default_config(&config_dir).unwrap();
            println!("Sample configuration file created in {:?}, please edit and add your legacy Slack token.", config_dir);
            std::process::exit(1);
        }
    };

    debug!("Checking Slack legacy token is not empty...");
    let token = if config.token != "" {
        config.token.clone()
    } else {
        println!("You must copy your Slack legacy token to configuration file.");
        std::process::exit(1);
    };

    info!("Requesting public ip...");
    let ip: ::std::net::IpAddr = match my_internet_ip::get() {
        Ok(ip) => ip,
        Err(e) => panic!("Could not get public IP: {:#?}", e),
    };

    info!("Computing status...");
    let status = get_status_from(config, &ip);

    info!("Updating Slack status...");
    let client = reqwest::Client::new();
    let res: reqwest::Response = match client
        .post("https://slack.com/api/users.profile.set")
        .bearer_auth(token)
        .json(&json!({
                "profile": {
                    "status_text": status.text,
                    "status_emoji": status.emoji,
                    "status_expiration": 0
                }
            })).send()
    {
        Ok(res) => res,
        Err(e) => panic!("Failed to change status: {:?}", e),
    };

    debug!("{:#?}", res);
}
