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
use std::fmt;
use std::io::prelude::*;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::prelude::*;
use chrono::Duration;
use directories::ProjectDirs;
use reqwest::blocking::*;

pub type BoxResult<T> = Result<T,Box<dyn Error>>;
pub type ReqwestResult = Result<reqwest::blocking::Response, reqwest::Error>;

/// Slack Status, as sent to the API.
#[derive(Serialize, Deserialize, Clone)]
pub struct Status {
    pub text: String,
    pub emoji: String,
    pub expire_after_hours: Option<i64>,
}

/// A Location matches an IP address (either IPv4 or IPv6) with a Status.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Location {
    pub ip: IpAddr,
    pub text: String,
    pub emoji: String,
    pub expire_after_hours: Option<i64>,
}

/// Config as read/write in TOML file.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub token: String,
    pub ip_request_address: Option<String>,
    pub locations: Vec<Location>,
    pub defaults: Option<Status>,
}

pub struct SlackStatus<'a> {
    client: Client,
    pub config: &'a Config,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.emoji, self.text)
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} => {} {}", self.ip, self.emoji, self.text)
    }
}

impl Config {
    /// Get the configuration file path either provided by the user or look at
    /// default location:
    ///
    /// * Linux: /home/alice/.config/slack-status/config.toml
    /// * Mac: /Users/Alice/Library/Preferences/com.nsd.slack-status/config.toml
    /// * Windows: C:\Users\Alice\AppData\Roaming\nsd\slack-status\config\config.toml
    fn get_file_path(path: Option<&str>) -> Option<PathBuf> {
        // User-provided configuration path.
        if let Some(path) = path {
            debug!("Looking for configuration file in: {:?}", path);
            Some(PathBuf::from(path))
        // Default OS location configuration path.
        } else if let Some(proj_dirs) = ProjectDirs::from("com", "nsd", "slack-status") {
            debug!("Looking for configuration file in: {:?}", proj_dirs.config_dir());
            Some(proj_dirs.config_dir().to_path_buf().join("config.toml"))
        } else {
            warn!("Cannot find configuration directory.");
            None
        }
    }

    /// Read configuration either from path provided by the user or look at
    /// default location.
    pub fn read(path: Option<&str>) -> BoxResult<Option<Config>> {
        if let Some(config_file_path) = Config::get_file_path(path) {
            match File::open(config_file_path) {
                Ok(mut f) => {
                    let mut contents = String::new();
                    f.read_to_string(&mut contents)
                        .expect("something went wrong reading the file");

                    let config = match toml::from_str(contents.as_str()) {
                        Ok(c) => c,
                        Err(e) => bail!("Deserialization error: {}", e),
                    };

                    Ok(Some(config))
                }
                Err(e) => {
                    warn!("Cannot open configuration file at: {}.", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Save configuration either at path provided by the user or at default
    /// location.
    pub fn save(&self, path: Option<&str>) -> BoxResult<()> {
        if let Some(config_file_path) = Config::get_file_path(path) {
            match File::create(config_file_path) {
                Ok(mut f) => {
                    let config = match toml::to_string_pretty(self) {
                        Ok(c) => c,
                        Err(e) => bail!("Serialization error: {}", e),
                    };
                    match f.write_all(config.as_bytes()) {
                        Ok(_) => {
                            info!("Configuration file saved");
                            Ok(())
                        },
                        Err(e) => bail!(e),
                    }
                }
                Err(e) => {
                    bail!("Cannot open configuration file at: {}.", e);
                }
            }
        } else {
            bail!("Cannot find configuration file.");
        }
    }
}

impl<'a> SlackStatus<'a> {
    pub fn from(config: &'a Config) -> BoxResult<SlackStatus> {
        if config.token.is_empty() {
            bail!("You must copy your Slack token to configuration file.");
        };

        Ok(SlackStatus {
            client: Client::new(),
            config: config,
        })
    }

    /// Request current Slack status.
    pub fn get_slack_status(&self) -> ReqwestResult {
        debug!("Requesting Slack status...");
        self.client.get("https://slack.com/api/users.profile.get")
            .bearer_auth(&self.config.token)
            .send()
    }

    /// Set Slack status.
    pub fn set_slack_status(&self, status: Status) -> ReqwestResult {
        debug!("Updating Slack status...");
        self.client.post("https://slack.com/api/users.profile.set")
            .bearer_auth(&self.config.token)
            .json(&json!({
                    "profile": {
                        "status_text": status.text,
                        "status_emoji": status.emoji,
                        "status_expiration": Utc::now().timestamp() +
                            Duration::hours(
                                status.expire_after_hours.unwrap_or(1))
                            .num_seconds(),
                    }
                }))
            .send()
    }

    /// Compute Slack status (currently only based on current location).
    pub fn status_from(&self, ip: &IpAddr) -> Status {
        match self.status_from_location(ip) {
            Some(status) => status,
            None => self.config.defaults.clone().unwrap_or(Status {
                text: "commuting".to_string(),
                emoji: ":mountain_railway:".to_string(),
                expire_after_hours: Some(1),
            }),
        }
    }

    /// Get status from configured locations and current public IP.
    pub fn status_from_location(&self, ip: &IpAddr) -> Option<Status> {
        let statuses: Vec<&Location> = self.config.locations.iter()
            .filter(|l| l.ip == *ip)
            .collect();

        match statuses.len() {
            0 => None,
            1 => {
                debug!("{} => {}", ip, statuses[0].text);
                Some(Status {
                    text: statuses[0].text.clone(),
                    emoji: statuses[0].emoji.clone(),
                    expire_after_hours: None,
                })
            },
            _ => {
                error!("Configuration error: several locations match your IP!");
                None
            },
        }
    }

    /// Get current public IP address.
    pub fn get_public_ip(&self) -> BoxResult<IpAddr> {
        let url = match &self.config.ip_request_address {
            Some(u) => u.clone(),
            None => "https://ip.clara.net".to_string(),
        };

        debug!("Requesting public ip to {}...", url);

        let resp = match self.client.get(url.as_str()).send() {
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
