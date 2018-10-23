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

#[derive(Deserialize)]
struct Location {
    ip: IpAddr,
    text: String,
    emoji: String,
}

#[derive(Deserialize)]
struct Config {
    token: String,
    locations: Vec<Location>,
}

fn get_config() -> Option<Config> {
    // Get standard configuration directory:
    // * Linux: /home/alice/.config/slack-status
    // * Mac: /Users/Alice/Library/Preferences/com.nsd.slack-status
    // * Windows: C:\Users\Alice\AppData\Roaming\nsd\slack-status\config
    if let Some(proj_dirs) = ProjectDirs::from("com", "nsd", "slack-status") {
        let config_dir = proj_dirs.config_dir();
        info!("Looking for configuration file in: {:?}", config_dir);

        let config_file_path = config_dir.join("config.toml");

        let mut f = File::open(config_file_path).expect("file not found");

        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");

        let config = toml::from_str(contents.as_str()).unwrap();

        Some(config)
    } else {
        panic!("Cannot find configuration directory.");
    }
}

fn main() {
    env_logger::init();

    let config = match get_config() {
        Some(config) => config,
        None => panic!("Can't find configuration file."),
    };

    info!("Requesting public ip...");
    let ip: ::std::net::IpAddr = match my_internet_ip::get() {
        Ok(ip) => ip,
        Err(e) => panic!("Could not get public IP: {:#?}", e),
    };

    let mut status_text: String = "en déplacement".to_string();
    let mut status_emoji: String = ":mountain_railway:".to_string();

    for location in config.locations {
        if location.ip == ip {
            info!("{} => {}", location.ip, location.text);
            status_text = location.text;
            status_emoji = location.emoji;
        }
    }

    let client = reqwest::Client::new();
    let res: reqwest::Response = match client
        .post("https://slack.com/api/users.profile.set")
        .bearer_auth(config.token)
        .json(&json!({
                "profile": {
                    "status_text": status_text,
                    "status_emoji": status_emoji,
                    "status_expiration": 0
                }
            })).send()
    {
        Ok(res) => res,
        Err(e) => panic!("Failed to change status: {:?}", e),
    };

    debug!("{:#?}", res);
}
