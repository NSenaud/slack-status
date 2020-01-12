use std::fs::File;
use std::error::Error;
use std::fmt;
use std::fs::create_dir_all;
use std::io::prelude::*;
use std::net::IpAddr;
use std::path::PathBuf;

use super::location::Location;

use directories::ProjectDirs;

type BoxResult<T> = Result<T,Box<dyn Error>>;

/// Slack Status, as sent to the API.
#[derive(Serialize, Deserialize, Clone)]
pub struct StatusConfig {
    pub text: String,
    pub emoji: String,
    pub expire_after_hours: Option<i64>,
}

/// Config, as read/write in configuration TOML file.
///
/// * token: Slack token, must have r/w right on user profile.
/// * ip_request_address: URL to request public IP address.
/// * locations: List of Location to set profile.
/// * ignore_ips: List of public IPs to ignore when setting status, such as
///               VPNs output addresses. In this case the cached status is
///               used instead.
/// * defaults: Status to use when you have no status associated to location.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub token: String,
    pub ip_request_address: Option<String>,
    pub ignore_ips: Vec<IpAddr>,
    pub locations: Vec<Location>,
    pub defaults: Option<StatusConfig>,
}

impl fmt::Display for StatusConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.emoji, self.text)
    }
}

impl Config {
    /// Create minimal config with token.
    pub fn with(token: String) -> Config {
        Config {
            token: token,
            ip_request_address: None,
            ignore_ips: Vec::<IpAddr>::new(),
            locations: Vec::<Location>::new(),
            defaults: None,
        }
    }

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
