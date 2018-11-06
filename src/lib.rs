#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
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
pub struct Status {
    pub text: String,
    pub emoji: String,
}

#[derive(Deserialize)]
pub struct Location {
    pub ip: IpAddr,
    pub text: String,
    pub emoji: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub token: String,
    pub defaults: Option<Status>,
    pub locations: Vec<Location>,
}

/// Get configuration from standard configuration directory:
///
/// * Linux: /home/alice/.config/slack-status/config.toml
/// * Mac: /Users/Alice/Library/Preferences/com.nsd.slack-status/config.toml
/// * Windows: C:\Users\Alice\AppData\Roaming\nsd\slack-status\config\config.toml
pub fn get_config_dir() -> Option<std::path::PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "nsd", "slack-status") {
        Some(proj_dirs.config_dir().to_path_buf())
    } else {
        None
    }
}

pub fn get_config() -> Option<Config> {
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

pub fn create_default_config(config_dir: &std::path::PathBuf) -> std::io::Result<()> {
    info!("Create sample config.toml at: {:?}", config_dir);

    let sample = include_str!("config.toml.sample");

    std::fs::create_dir_all(config_dir)?;
    let mut f = File::create(config_dir.join("config.toml"))?;
    f.write_all(sample.as_bytes())?;

    f.sync_all()?;
    Ok(())
}

/// Get status from configuration locations and current public IP.
pub fn get_status_from_location(locations: &[Location], ip: &IpAddr) -> Option<Status> {
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
pub fn get_status_from(config: Config, ip: &IpAddr) -> Status {
    match get_status_from_location(&config.locations, ip) {
        Some(status) => status,
        None => config.defaults.unwrap_or(Status {
            text: "on the move".to_string(),
            emoji: ":mountain_railway:".to_string(),
        }),
    }
}
