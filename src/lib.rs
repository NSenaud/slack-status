#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate simple_error;

use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::prelude::*;
use directories::ProjectDirs;

type BoxResult<T> = Result<T,Box<dyn Error>>;

#[derive(Deserialize, Clone)]
pub struct Status {
    pub text: String,
    pub emoji: String,
}

#[derive(Deserialize, Clone)]
pub struct Location {
    pub ip_addresses: Vec<IpAddr>,
    pub text: String,
    pub emoji: String,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub token: String,
    pub ip_request_address: Option<String>,
    pub defaults: Option<Status>,
    pub locations: Vec<Location>,
}

pub struct SlackStatus {
    client: reqwest::Client,
    config: Config,
}

impl Config {
    pub fn read(path: Option<&str>) -> Option<Config> {
        let config_file_path;

        if let Some(path) = path {
            config_file_path = PathBuf::from(path)
        } else if let Some(config_dir) = get_config_dir() {
            info!("Looking for configuration file in: {:?}", config_dir);

            config_file_path = config_dir.join("config.toml");
        } else {
            warn!("Cannot find configuration directory.");
            return None
        }

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
    }
}

impl SlackStatus {
    pub fn from(config: Config) -> BoxResult<SlackStatus> {
        if config.token.is_empty() {
            bail!("You must copy your Slack legacy token to configuration file.");
        };

        Ok(SlackStatus {
            client: reqwest::Client::new(),
            config: config,
        })
    }

    pub fn set_slack_status(&self, status: Status) -> Result<reqwest::Response, reqwest::Error> {
        debug!("Updating Slack status...");
        self.client.post("https://slack.com/api/users.profile.set")
            .bearer_auth(&self.config.token)
            .json(&json!({
                    "profile": {
                        "status_text": status.text,
                        "status_emoji": status.emoji,
                        "status_expiration": Utc::now().timestamp() + 3600,
                    }
                }))
        .send()
    }

    pub fn status_from(&self, ip: &IpAddr) -> Status {
        match self.status_from_location(ip) {
            Some(status) => status,
            None => self.config.defaults.clone().unwrap_or(Status {
                text: "on the move".to_string(),
                emoji: ":mountain_railway:".to_string(),
            }),
        }
    }

    /// Get status from configuration locations and current public IP.
    pub fn status_from_location(&self, ip: &IpAddr) -> Option<Status> {
        let statuses: Vec<&Location> = self.config.locations.iter()
            .filter(|l| l.ip_addresses.iter()
                .any(|i| i == ip))
            .collect();

        match statuses.len() {
            0 => None,
            1 => {
                info!("{} => {}", ip, statuses[0].text);
                Some(Status {
                    text: statuses[0].text.clone(),
                    emoji: statuses[0].emoji.clone(),
                })
            },
            _ => {
                error!("Configuration error: several locations match your IP!");
                None
            },
        }
    }

    pub fn get_public_ip(&self) -> BoxResult<IpAddr> {
        let url = match &self.config.ip_request_address {
            Some(u) => u.clone(),
            None => "https://ip.clara.net".to_string(),
        };

        debug!("Requesting public ip to {}...", url);

        let mut resp = match self.client.get(url.as_str()).send() {
            Ok(r) => r,
            Err(e) => bail!(format!("Request error: {}", e)),
        };

        if resp.status().is_success() {
            return match IpAddr::from_str(&resp.text().unwrap()) {
                Ok(ip) => Ok::<IpAddr, Box<dyn Error>>(ip),
                Err(e) => bail!(format!("Cannot parse IP: {}", e)),
            }
        }

        bail!(format!("Request error, status is: {}", resp.status()))
    }
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

pub fn create_default_config(config_dir: &std::path::PathBuf) -> std::io::Result<()> {
    info!("Create sample config.toml at: {:?}", config_dir);

    let sample = include_str!("config.toml.sample");

    std::fs::create_dir_all(config_dir)?;
    let mut f = File::create(config_dir.join("config.toml"))?;
    f.write_all(sample.as_bytes())?;

    f.sync_all()?;
    Ok(())
}
