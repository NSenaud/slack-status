#[macro_use] extern crate log;
#[macro_use] extern crate serde_json;
extern crate env_logger;
extern crate my_internet_ip;
extern crate reqwest;

use std::net::{IpAddr, Ipv4Addr};

struct Location {
    ip: IpAddr,
    text: String,
    emoji: String,
}

struct Config {
    token: String,
    locations: Vec<Location>,
}

fn get_config() -> Config {
    Config {
        token: "".to_string(),
        locations: vec![
            Location {
                ip: IpAddr::V4(Ipv4Addr::new(176, 187, 98, 85)),
                text: "Maison".to_string(),
                emoji: ":house:".to_string(),
            },
            Location {
                ip: IpAddr::V4(Ipv4Addr::new(78, 193, 77, 48)),
                text: "ipso Saint-Martin".to_string(),
                emoji: ":ipso:".to_string(),
            },
            Location {
                ip: IpAddr::V4(Ipv4Addr::new(81, 250, 187, 198)),
                text: "ipso Nation".to_string(),
                emoji: ":ipso:".to_string(),
            }
        ]
    }
}

fn main() {
    env_logger::init();

    let config = get_config();

    info!("Requesting public ip...");
    let ip: ::std::net::IpAddr = match my_internet_ip::get() {
        Ok(ip) => ip,
        Err(e) => panic!("Could not get public IP: {:#?}", e),
    };

    let mut status_text: String = "en deplacement".to_string();
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
