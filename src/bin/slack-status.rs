#[macro_use]
extern crate log;
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate directories;
extern crate env_logger;
extern crate my_internet_ip;
extern crate reqwest;
extern crate toml;
extern crate slack_status;

use slack_status::*;

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
