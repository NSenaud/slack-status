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
use std::fs::create_dir_all;
use std::io::prelude::*;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::prelude::*;
use chrono::Duration;
use directories::ProjectDirs;
use reqwest::blocking::*;
use serde_json::Value;

pub type BoxResult<T> = Result<T,Box<dyn Error>>;
pub type ReqwestResult = Result<reqwest::blocking::Response, reqwest::Error>;

/// Slack Status, as sent to the API.
#[derive(Serialize, Deserialize, Clone)]
pub struct StatusConfig {
    pub text: String,
    pub emoji: String,
    pub expire_after_hours: Option<i64>,
}

/// Slack Status, as sent to the API.
#[derive(Serialize, Deserialize, Clone)]
pub struct StatusCache {
    pub text: String,
    pub emoji: String,
    pub expiration: i64,
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
    pub defaults: Option<StatusConfig>,
}

/// Cache status and keep track if it was manually set.
#[derive(Serialize, Deserialize, Clone)]
pub struct Cache {
    pub status: StatusCache,
    pub manually_set: bool,
}

pub struct SlackStatus<'a> {
    client: Client,
    pub config: &'a Config,
}

impl fmt::Display for StatusConfig {
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
            let config_dir = proj_dirs.config_dir();
            debug!("Looking for configuration file in: {:?}", config_dir);

            if !config_dir.to_path_buf().exists() {
                debug!("Config directory does not exists, creating it.");
                create_dir_all(config_dir.to_str().unwrap()).unwrap();
            }

            Some(config_dir.to_path_buf().join("config.toml"))
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

impl Cache {
    /// Get the cache file path either provided by the user or look at
    /// default location:
    ///
    /// * Linux: /home/alice/.cache/slack-status/status.json
    /// * Mac: /Users/Alice/Library/Caches/com.nsd.slack-status/status.json
    /// * Windows: C:\Users\Alice\AppData\Roaming\nsd\slack-status\cache\status.json
    fn get_file_path() -> Option<PathBuf> {
        // Default OS location configuration path.
        if let Some(proj_dirs) = ProjectDirs::from("com", "nsd", "slack-status") {
            let cache_dir = proj_dirs.cache_dir();
            debug!("Looking for cache file in: {:?}", cache_dir);

            if !cache_dir.to_path_buf().exists() {
                debug!("Cache directory does not exists, creating it.");
                create_dir_all(cache_dir.to_str().unwrap()).unwrap();
            }

            Some(cache_dir.to_path_buf().join("status.json"))
        } else {
            warn!("Cannot find application cache directory.");
            None
        }
    }

    /// Read cache file from default OS location.
    pub fn read() -> BoxResult<Option<Cache>> {
        if let Some(cache_file_path) = Cache::get_file_path() {
            match File::open(cache_file_path) {
                Ok(mut f) => {
                    let mut contents = String::new();
                    f.read_to_string(&mut contents)
                        .expect("something went wrong reading the file");

                    let cache = match serde_json::from_str(contents.as_str()) {
                        Ok(c) => c,
                        Err(e) => bail!("Deserialization error: {}", e),
                    };

                    Ok(Some(cache))
                }
                Err(e) => {
                    warn!("Cannot read cache directory: {}.", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Save cache file at default OS location.
    pub fn save(&self) -> BoxResult<()> {
        if let Some(cache_file_path) = Cache::get_file_path() {
            match File::create(cache_file_path) {
                Ok(mut f) => {
                    debug!("Cache file: {:#?}", f);

                    let cache = match serde_json::to_string(self) {
                        Ok(c) => c,
                        Err(e) => bail!("Serialization error: {}", e),
                    };
                    debug!("Cache file content: {}", cache);

                    match f.write_all(cache.as_bytes()) {
                        Ok(_) => {
                            info!("Cache file saved");
                            Ok(())
                        },
                        Err(e) => bail!(e),
                    }
                }
                Err(e) => {
                    bail!("Cannot read cache directory: {}.", e)
                }
            }
        } else {
            bail!("Cannot find application cache directory.");
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
    pub fn get_slack_status(&self) -> BoxResult<Option<StatusCache>>{
        debug!("Requesting Slack status...");
        let res = match self.client.get("https://slack.com/api/users.profile.get")
            .bearer_auth(&self.config.token)
            .send()
        {
            Ok(res) => match res.text() {
                Ok(r) => r,
                Err(e) => bail!("Failed to get your Slack status: {:?}", e),
            },
            Err(e) => bail!("Failed to get your Slack status: {:?}", e),
        };
        debug!("{:#?}", res);

        let value: Value = match serde_json::from_str(&res) {
            Ok(v) => v,
            Err(e) => bail!("Cannot deserialize: {}", e),
        };

        let text = value["profile"]["status_text"].to_string();
        let emoji = value["profile"]["status_emoji"].to_string();
        let expiration = value["profile"]["status_expiration"].as_i64().unwrap();

        Ok(Some(StatusCache {
            text: text.trim_matches('"').to_string(),
            emoji: emoji.trim_matches('"').to_string(),
            expiration: expiration,
        }))
    }

    /// Set Slack status.
    pub fn set_slack_status(&self, status: &StatusConfig, manually_set: bool) -> BoxResult<()> {
        // If the status have been set manually and haven't expired yet, then
        // it won't be automatically updated.
        // TODO: Add a reset cache command
        if !manually_set {
            let cache_file = match Cache::read() {
                Ok(c) => c,
                Err(e) => {
                    error!("Cannot read cache: {}", e);
                    None
                },
            };

            if let Some(cache) = cache_file {
                if cache.manually_set &&
                    (cache.status.expiration > Utc::now().timestamp())
                {
                    info!("Status set manually, too soon to update automatically.");
                    return Ok(())
                }
            }
        }

        debug!("Updating Slack status...");
        let expiration = match status.expire_after_hours {
            Some(h) =>
                Utc::now().timestamp() + Duration::hours(h).num_seconds(),
            None => 0,
        };
        let data = json!({
                    "profile": {
                        "status_text": status.text,
                        "status_emoji": status.emoji,
                        "status_expiration": expiration,
                    }
                });
        debug!("data: {}", &data);

        let res = self.client.post("https://slack.com/api/users.profile.set")
            .bearer_auth(&self.config.token)
            .json(&data)
            .send();
        debug!("{:#?}", res);

        // Cache status.
        let cache = Cache {
            status: StatusCache {
                text: status.text.clone(),
                emoji: status.emoji.clone(),
                expiration: expiration,
            },
            manually_set: manually_set,
        };
        cache.save()?;

        Ok(())
    }

    /// Compute Slack status (currently only based on current location).
    pub fn status_from(&self, ip: &IpAddr) -> StatusConfig {
        match self.status_from_location(ip) {
            Some(status) => status,
            None => self.config.defaults.clone().unwrap_or(StatusConfig {
                text: "commuting".to_string(),
                emoji: ":mountain_railway:".to_string(),
                expire_after_hours: Some(1),
            }),
        }
    }

    /// Get status from configured locations and current public IP.
    pub fn status_from_location(&self, ip: &IpAddr) -> Option<StatusConfig> {
        let statuses: Vec<&Location> = self.config.locations.iter()
            .filter(|l| l.ip == *ip)
            .collect();

        match statuses.len() {
            0 => None,
            1 => {
                debug!("{} => {}", ip, statuses[0].text);
                Some(StatusConfig {
                    text: statuses[0].text.clone(),
                    emoji: statuses[0].emoji.clone(),
                    expire_after_hours: statuses[0].expire_after_hours.clone(),
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
