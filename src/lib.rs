#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate simple_error;

pub mod cache;
pub mod config;
pub mod location;

use std::error::Error;
use std::net::IpAddr;
use std::str::FromStr;

use chrono::prelude::*;
use chrono::Duration;
use reqwest::blocking::*;
use serde_json::Value;

pub use cache::{Cache, StatusCache};
pub use config::{Config, StatusConfig};
pub use location::Location;

pub type BoxResult<T> = Result<T,Box<dyn Error>>;
pub type ReqwestResult = Result<reqwest::blocking::Response, reqwest::Error>;

pub struct SlackStatus<'a> {
    client: Client,
    pub config: &'a Config,
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

    // TODO: UX: make it clear when status come from cache.

    /// Compute Slack status based on current location.
    pub fn status_from(&self, ip: &IpAddr) -> StatusConfig {
        // Check if location is set to be ignored, in that case get status from
        // cache
        if self.config.ignore_ips.iter().any(|i| i == ip) {
            let cache_file = match Cache::read() {
                Ok(c) => c,
                Err(e) => {
                    error!("Cannot read cache: {}", e);
                    None
                },
            };

            if let Some(cache) = cache_file {
                return StatusConfig {
                    text: cache.status.text,
                    emoji: cache.status.emoji,
                    expire_after_hours: None,
                }
            }
        }

        // Else try to get location from location
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
